//! Controlador del jugador en forma de cápsula ("píldora") 3D con soporte móvil y teclado.
//!
//! Maneja movimiento físico (`CharacterBody3D`), agachado real con reducción de colisionador
//! para cubrirse detrás de barreras y sacos de arena, salto, cámara táctica sobre el hombro
//! (con opción de cambiar a primera persona), recuperación de retroceso y recepción de daño.

use godot::classes::{
    Camera3D, CapsuleShape3D, CharacterBody3D, CollisionObject3D, CollisionShape3D,
    ICharacterBody3D, Input, Node3D, SpringArm3D,
};
use godot::prelude::*;

use crate::combat_math::calculate_effective_spread_deg;
use crate::weapon_rig::WeaponRig;

/// Controlador principal de la cápsula del jugador.
#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct PlayerCapsule {
    base: Base<CharacterBody3D>,

    #[init(val = 100.0)]
    max_health: f32,
    #[init(val = 100.0)]
    health: f32,
    #[init(val = 5.4)]
    walk_speed: f32,
    #[init(val = 7.2)]
    sprint_speed: f32,
    #[init(val = 3.1)]
    crouch_speed: f32,
    #[init(val = 5.2)]
    jump_velocity: f32,
    #[init(val = 14.5)]
    gravity: f32,
    #[init(val = 0.0032)]
    aim_sensitivity: f32,

    move_input: Vector2,
    is_firing: bool,
    is_crouching: bool,
    is_sprinting: bool,
    is_dead: bool,
    first_person_mode: bool,
    crouch_blend: f32,

    yaw: f32,
    pitch: f32,
    recoil_recover_pitch: f32,
    recoil_recover_yaw: f32,
    controls_enabled: bool,

    collision_shape: Option<Gd<CollisionShape3D>>,
    visual_pivot: Option<Gd<Node3D>>,
    camera_pivot: Option<Gd<Node3D>>,
    spring_arm: Option<Gd<SpringArm3D>>,
    camera: Option<Gd<Camera3D>>,
    first_person_anchor: Option<Gd<Node3D>>,
    weapon_mount: Option<Gd<Node3D>>,
    weapon_rig: Option<Gd<WeaponRig>>,
}

#[godot_api]
impl ICharacterBody3D for PlayerCapsule {
    fn ready(&mut self) {
        self.health = self.max_health;
        self.controls_enabled = false;

        self.collision_shape = self.base().try_get_node_as::<CollisionShape3D>("CollisionShape3D");
        self.visual_pivot = self.base().try_get_node_as::<Node3D>("VisualPivot");
        self.camera_pivot = self.base().try_get_node_as::<Node3D>("CameraPivot");
        self.spring_arm = self.base().try_get_node_as::<SpringArm3D>("CameraPivot/SpringArm3D");
        self.camera = self.base().try_get_node_as::<Camera3D>("CameraPivot/SpringArm3D/Camera3D");
        self.first_person_anchor = self.base().try_get_node_as::<Node3D>("CameraPivot/FirstPersonAnchor");
        self.weapon_mount = self.base().try_get_node_as::<Node3D>("WeaponMount");
        self.weapon_rig = self.base().try_get_node_as::<WeaponRig>("WeaponMount/WeaponRig");

        let player_rid = self.base().get_rid();
        if let Some(mut arm) = self.spring_arm.clone() {
            arm.add_excluded_object(player_rid);
        }

        if let Some(mut rig) = self.weapon_rig.clone() {
            let body_obj: Gd<CollisionObject3D> = self.to_gd().upcast();
            rig.bind_mut().set_shooter_body(Some(body_obj));

            // Conecta el evento de impacto del rifle hacia la señal de confirmación de golpe del jugador.
            let self_obj = self.to_gd();
            let callable = Callable::from_object_method(&self_obj, "_on_weapon_hit_target");
            if !rig.is_connected("hit_target", &callable) {
                rig.connect("hit_target", &callable);
            }
        }
    }

    fn physics_process(&mut self, delta: f64) {
        let dt = delta as f32;

        if self.is_dead {
            return;
        }

        // Lee acciones de teclado como respaldo cuando se prueba en escritorio.
        let mut combined_move = self.move_input;
        let mut wants_fire = self.is_firing;

        if self.controls_enabled {
            let input = Input::singleton();
            let kb_x = input.get_axis("move_left", "move_right");
            let kb_y = input.get_axis("move_forward", "move_backward");
            if kb_x.abs() > 0.05 || kb_y.abs() > 0.05 {
                combined_move = Vector2::new(kb_x, kb_y);
            }
            if input.is_action_pressed("fire") {
                wants_fire = true;
            }
            if input.is_action_just_pressed("reload") {
                self.trigger_reload();
            }
            if input.is_action_just_pressed("crouch") {
                self.is_crouching = !self.is_crouching;
            }
            if input.is_action_just_pressed("jump") {
                self.trigger_jump();
            }
        } else {
            combined_move = Vector2::ZERO;
            wants_fire = false;
        }

        if combined_move.length() > 1.0 {
            combined_move = combined_move.normalized();
        }

        // Transición suave de agachado para ajustar altura real de cápsula y cobertura.
        let target_crouch = if self.is_crouching { 1.0 } else { 0.0 };
        self.crouch_blend += (target_crouch - self.crouch_blend) * (dt * 11.0).min(1.0);
        self.apply_stance_heights();

        // Movimiento relativo a la orientación horizontal de la cápsula.
        let mut vel = self.base().get_velocity();
        let on_floor = self.base().is_on_floor();
        if !on_floor {
            vel.y -= self.gravity * dt;
        } else if vel.y < 0.0 {
            vel.y = -0.1;
        }

        let xform = self.base().get_global_transform();
        let forward = -xform.basis.col_c();
        let right = xform.basis.col_a();
        let flat_fwd = Vector3::new(forward.x, 0.0, forward.z).normalized_or_zero();
        let flat_right = Vector3::new(right.x, 0.0, right.z).normalized_or_zero();

        let move_dir = (flat_right * combined_move.x) - (flat_fwd * combined_move.y);
        let speed = if self.is_crouching {
            self.crouch_speed
        } else if self.is_sprinting && combined_move.y < -0.55 {
            self.sprint_speed
        } else {
            self.walk_speed
        };

        let target_vx = move_dir.x * speed;
        let target_vz = move_dir.z * speed;
        let accel = if on_floor { 14.0 } else { 4.5 };
        vel.x += (target_vx - vel.x) * (dt * accel).min(1.0);
        vel.z += (target_vz - vel.z) * (dt * accel).min(1.0);

        self.base_mut().set_velocity(vel);
        self.base_mut().move_and_slide();

        // Recuperación suave del retroceso visual de cámara.
        if self.recoil_recover_pitch.abs() > 0.0001 {
            let decay = self.recoil_recover_pitch * (dt * 9.0).min(1.0);
            self.pitch = (self.pitch - decay).clamp(-1.15, 1.10);
            self.recoil_recover_pitch -= decay;
        }
        if self.recoil_recover_yaw.abs() > 0.0001 {
            let decay_y = self.recoil_recover_yaw * (dt * 9.0).min(1.0);
            self.yaw -= decay_y;
            self.recoil_recover_yaw -= decay_y;
        }

        self.apply_aim_transforms();

        // Disparo continuo si el botón táctil o tecla están presionados.
        if wants_fire && self.controls_enabled {
            let speed_ratio = Vector2::new(vel.x, vel.z).length() / self.walk_speed.max(0.1);
            self.fire_weapon_step(speed_ratio);
        }
    }
}

#[godot_api]
impl PlayerCapsule {
    #[signal]
    fn health_changed(current: f32, max_hp: f32);

    #[signal]
    fn died();

    #[signal]
    fn hit_confirmed(damage: f32);

    #[signal]
    fn took_damage(amount: f32, attacker_pos: Vector3);

    #[func]
    fn _on_weapon_hit_target(&mut self, damage: f32, _hit_pos: Vector3) {
        self.signals().hit_confirmed().emit(damage);
    }

    #[func]
    pub fn set_controls_enabled(&mut self, enabled: bool) {
        self.controls_enabled = enabled;
        if !enabled {
            self.move_input = Vector2::ZERO;
            self.is_firing = false;
            self.is_sprinting = false;
        }
    }

    #[func]
    pub fn set_move_input(&mut self, input: Vector2) {
        self.move_input = input;
    }

    #[func]
    pub fn add_look_delta(&mut self, delta: Vector2) {
        if !self.controls_enabled || self.is_dead {
            return;
        }
        self.yaw -= delta.x * self.aim_sensitivity;
        self.pitch = (self.pitch - delta.y * self.aim_sensitivity).clamp(-1.15, 1.10);
        self.apply_aim_transforms();
    }

    #[func]
    pub fn set_firing(&mut self, firing: bool) {
        self.is_firing = firing;
    }

    #[func]
    pub fn set_crouching(&mut self, crouching: bool) {
        self.is_crouching = crouching;
    }

    #[func]
    pub fn is_player_crouching(&self) -> bool {
        self.is_crouching
    }

    #[func]
    pub fn set_sprinting(&mut self, sprinting: bool) {
        self.is_sprinting = sprinting;
    }

    #[func]
    pub fn trigger_jump(&mut self) {
        if !self.controls_enabled || self.is_dead {
            return;
        }
        if self.base().is_on_floor() {
            self.is_crouching = false;
            let mut vel = self.base().get_velocity();
            vel.y = self.jump_velocity;
            self.base_mut().set_velocity(vel);
        }
    }

    #[func]
    pub fn trigger_reload(&mut self) {
        if !self.controls_enabled || self.is_dead {
            return;
        }
        if let Some(mut rig) = self.weapon_rig.clone() {
            rig.bind_mut().try_reload();
        }
    }

    #[func]
    pub fn set_aim_sensitivity(&mut self, sens: f32) {
        self.aim_sensitivity = sens.clamp(0.0008, 0.012);
    }

    #[func]
    pub fn set_camera_mode_first_person(&mut self, fps: bool) {
        self.first_person_mode = fps;
        if let Some(mut arm) = self.spring_arm.clone() {
            arm.set_length(if fps { 0.0 } else { 2.35 });
            let pos = if fps {
                Vector3::new(0.0, 0.08, -0.18)
            } else {
                Vector3::new(0.54, 0.14, 0.0)
            };
            arm.set_position(pos);
        }
    }

    #[func]
    pub fn get_weapon_rig(&self) -> Option<Gd<WeaponRig>> {
        self.weapon_rig.clone()
    }

    #[func]
    pub fn get_health(&self) -> f32 {
        self.health
    }

    /// Reinicia posición, vida, rotación inicial mirando al enemigo y munición al empezar la ronda.
    #[func]
    pub fn reset_for_round(&mut self, spawn_pos: Vector3, face_target: Vector3) {
        self.is_dead = false;
        self.health = self.max_health;
        self.is_crouching = false;
        self.crouch_blend = 0.0;
        self.is_firing = false;
        self.move_input = Vector2::ZERO;
        self.recoil_recover_pitch = 0.0;
        self.recoil_recover_yaw = 0.0;

        self.base_mut().set_global_position(spawn_pos);
        self.base_mut().set_velocity(Vector3::ZERO);

        let to_target = Vector3::new(face_target.x - spawn_pos.x, 0.0, face_target.z - spawn_pos.z);
        if to_target.length_squared() > 0.001 {
            // En Godot -Z es frente, por tanto yaw = atan2(-dx, -dz).
            self.yaw = (-to_target.x).atan2(-to_target.z);
        } else {
            self.yaw = 0.0;
        }
        self.pitch = -0.02;
        self.apply_stance_heights();
        self.apply_aim_transforms();

        if let Some(mut rig) = self.weapon_rig.clone() {
            rig.bind_mut().reset_ammo();
        }

        let hp = self.health;
        let max_hp = self.max_health;
        self.signals().health_changed().emit(hp, max_hp);
    }

    /// Recibe impacto balístico del arma enemiga.
    #[func]
    pub fn apply_bullet_damage(&mut self, amount: f32, attacker_pos: Vector3) {
        if self.is_dead || amount <= 0.0 {
            return;
        }
        self.health = (self.health - amount).max(0.0);
        let hp = self.health;
        let max_hp = self.max_health;
        self.signals().health_changed().emit(hp, max_hp);
        self.signals().took_damage().emit(amount, attacker_pos);

        if self.health <= 0.0 {
            self.is_dead = true;
            self.is_firing = false;
            self.controls_enabled = false;
            self.signals().died().emit();
        }
    }

    fn apply_stance_heights(&mut self) {
        let blend = self.crouch_blend;
        let col_height = 1.80 - 0.62 * blend;

        if let Some(mut col) = self.collision_shape.clone() {
            col.set_position(Vector3::new(0.0, col_height * 0.5, 0.0));
            if let Some(shape) = col.get_shape() {
                if let Ok(mut capsule) = shape.try_cast::<CapsuleShape3D>() {
                    capsule.set_height(col_height);
                }
            }
        }
        if let Some(mut vis) = self.visual_pivot.clone() {
            vis.set_scale(Vector3::new(1.0, 1.0 - 0.34 * blend, 1.0));
        }
        if let Some(mut cam_piv) = self.camera_pivot.clone() {
            let cam_y = 1.48 - 0.50 * blend;
            cam_piv.set_position(Vector3::new(0.0, cam_y, 0.0));
        }
        if let Some(mut w_mount) = self.weapon_mount.clone() {
            let w_y = 1.24 - 0.42 * blend;
            let w_x = if self.first_person_mode { 0.22 } else { 0.31 };
            let w_z = if self.first_person_mode { -0.44 } else { -0.27 };
            w_mount.set_position(Vector3::new(w_x, w_y, w_z));
        }
    }

    fn apply_aim_transforms(&mut self) {
        let yaw = self.yaw;
        self.base_mut().set_rotation(Vector3::new(0.0, yaw, 0.0));
        if let Some(mut cam_piv) = self.camera_pivot.clone() {
            cam_piv.set_rotation(Vector3::new(self.pitch, 0.0, 0.0));
        }
        if let Some(mut w_mount) = self.weapon_mount.clone() {
            w_mount.set_rotation(Vector3::new(self.pitch, 0.0, 0.0));
        }
    }

    fn fire_weapon_step(&mut self, speed_ratio: f32) {
        let Some(cam) = self.camera.clone() else {
            return;
        };
        let Some(mut rig) = self.weapon_rig.clone() else {
            return;
        };

        let cam_xform = cam.get_global_transform();
        let aim_origin = cam_xform.origin;
        let aim_dir = -cam_xform.basis.col_c();

        let heat = rig.bind().get_recoil_heat();
        let spread = calculate_effective_spread_deg(0.75, speed_ratio, self.is_crouching, heat);
        let fired = rig.bind_mut().try_fire(aim_origin, aim_dir, spread);

        if fired {
            let (kick_pitch, kick_yaw) = rig.bind_mut().consume_recoil_kick();
            self.pitch = (self.pitch + kick_pitch).clamp(-1.15, 1.10);
            self.yaw += kick_yaw;
            self.recoil_recover_pitch += kick_pitch * 0.72;
            self.recoil_recover_yaw += kick_yaw * 0.55;
            self.apply_aim_transforms();
        }
    }
}
