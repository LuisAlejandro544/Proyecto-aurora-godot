//! Controlador del jugador en forma de cápsula ("píldora") 3D con movilidad táctica avanzada.
//!
//! Maneja movimiento físico (`CharacterBody3D`), agachado real con reducción de colisionador,
//! deslizamiento táctico y Slide Cancel, lanzamiento en plancha (Dolphin Dive), apuntado con mira
//! (ADS) con zoom FOV y recarga sincronizada en mira, lanzamiento de granadas de fragmentación,
//! cámara 1ª/3ª persona, recuperación de retroceso y recepción de daño.

use godot::classes::{
    input::MouseMode, Camera3D, CapsuleShape3D, CharacterBody3D, CollisionObject3D,
    CollisionShape3D, ICharacterBody3D, Input, Node3D, PackedScene, SpringArm3D,
};
use godot::prelude::*;

use crate::combat_math::{calculate_tactical_spread_deg, compute_slide_speed};
use crate::grenade::{find_animation_player_recursive, FragGrenade};
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
    #[init(val = 7.4)]
    sprint_speed: f32,
    #[init(val = 3.1)]
    crouch_speed: f32,
    #[init(val = 5.2)]
    jump_velocity: f32,
    #[init(val = 14.5)]
    gravity: f32,
    #[init(val = 0.0032)]
    aim_sensitivity: f32,
    #[init(val = 2)]
    max_grenades: i32,

    move_input: Vector2,
    is_firing: bool,
    is_crouching: bool,
    is_sprinting: bool,
    is_aiming: bool,
    is_dead: bool,
    first_person_mode: bool,

    crouch_blend: f32,
    ads_blend: f32,

    // Estado de Slide & Slide Cancel estilo Warzone
    is_sliding: bool,
    slide_timer: f32,
    #[init(val = 0.62)]
    slide_duration: f32,
    slide_cooldown: f32,
    slide_dir: Vector3,

    // Estado de Dolphin Dive ("lanzarse")
    is_diving: bool,
    dive_airborne: bool,
    dive_recover_timer: f32,
    dive_cooldown: f32,
    dive_dir: Vector3,

    // Granadas tácticas
    #[init(val = 2)]
    grenades_left: i32,
    grenade_cooldown: f32,
    grenade_scene: Option<Gd<PackedScene>>,
    explosion_scene: Option<Gd<PackedScene>>,
    fx_container: Option<Gd<Node3D>>,

    yaw: f32,
    pitch: f32,
    recoil_recover_pitch: f32,
    recoil_recover_yaw: f32,
    controls_enabled: bool,
    was_on_floor: bool,
    step_distance_accum: f32,
    last_stance_label: String,

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
        self.base_mut().add_to_group("combatants");
        self.health = self.max_health;
        self.grenades_left = self.max_grenades;
        self.controls_enabled = false;
        self.was_on_floor = true;
        self.step_distance_accum = 0.4;
        self.last_stance_label = "DE PIE".to_string();

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
        }
    }

    fn physics_process(&mut self, delta: f64) {
        let dt = delta as f32;

        if self.is_dead {
            return;
        }

        if self.slide_cooldown > 0.0 {
            self.slide_cooldown = (self.slide_cooldown - dt).max(0.0);
        }
        if self.dive_cooldown > 0.0 {
            self.dive_cooldown = (self.dive_cooldown - dt).max(0.0);
        }
        if self.grenade_cooldown > 0.0 {
            self.grenade_cooldown = (self.grenade_cooldown - dt).max(0.0);
        }

        let mut combined_move = self.move_input;
        let mut wants_fire = self.is_firing;
        let mut kb_sprint = false;

        if self.controls_enabled {
            let input = Input::singleton();
            let kb_x = input.get_axis("move_left", "move_right");
            let kb_y = input.get_axis("move_forward", "move_backward");
            if kb_x.abs() > 0.05 || kb_y.abs() > 0.05 {
                combined_move = Vector2::new(kb_x, kb_y);
            }
            if input.is_action_pressed("sprint") && combined_move.y < -0.35 {
                kb_sprint = true;
            }
            // Solo lee clics de ratón como disparo/mira cuando el cursor está capturado (PC);
            // en pantalla táctil móvil los toques emulan ratón y dispararían al girar la vista.
            if input.get_mouse_mode() == MouseMode::CAPTURED {
                if input.is_action_pressed("fire") {
                    wants_fire = true;
                }
                if input.is_action_just_pressed("aim") {
                    self.toggle_aiming();
                }
            }
            if input.is_action_just_pressed("reload") {
                self.trigger_reload();
            }
            if input.is_action_just_pressed("crouch") {
                let next_crouch = !self.is_crouching;
                self.set_crouching(next_crouch);
            }
            if input.is_action_just_pressed("jump") {
                self.trigger_jump();
            }
            if input.is_action_just_pressed("dive") {
                self.trigger_dive();
            }
            if input.is_action_just_pressed("grenade") {
                self.trigger_grenade();
            }
        } else {
            combined_move = Vector2::ZERO;
            wants_fire = false;
        }

        if combined_move.length() > 1.0 {
            combined_move = combined_move.normalized();
        }

        let effective_sprint =
            (self.is_sprinting || kb_sprint) && !self.is_aiming && !self.is_crouching && !self.is_diving;

        // Interpolación suave de postura (crouch/slide/dive) y apuntado ADS.
        let low_stance = self.is_crouching || self.is_sliding || self.is_diving;
        let target_crouch = if low_stance { 1.0 } else { 0.0 };
        let crouch_rate = if self.is_sliding || self.is_diving { 16.0 } else { 11.5 };
        self.crouch_blend += (target_crouch - self.crouch_blend) * (dt * crouch_rate).min(1.0);

        let target_ads = if self.is_aiming && !self.is_diving { 1.0 } else { 0.0 };
        self.ads_blend += (target_ads - self.ads_blend) * (dt * 13.0).min(1.0);

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

        if self.is_diving {
            if self.dive_airborne {
                vel.x += (self.dive_dir.x * 8.6 - vel.x) * (dt * 1.8).min(1.0);
                vel.z += (self.dive_dir.z * 8.6 - vel.z) * (dt * 1.8).min(1.0);
            } else {
                self.dive_recover_timer -= dt;
                vel.x += (0.0 - vel.x) * (dt * 9.5).min(1.0);
                vel.z += (0.0 - vel.z) * (dt * 9.5).min(1.0);
                if self.dive_recover_timer <= 0.0 {
                    self.is_diving = false;
                }
            }
        } else if self.is_sliding {
            self.slide_timer += dt;
            let slide_speed = compute_slide_speed(self.slide_timer, self.slide_duration, 9.8, 3.6);
            // Permite un ligero control lateral (steer) durante el deslizamiento.
            let steer_dir = (self.slide_dir + flat_right * (combined_move.x * 0.28)).normalized_or_zero();
            vel.x = steer_dir.x * slide_speed;
            vel.z = steer_dir.z * slide_speed;

            if self.slide_timer >= self.slide_duration || !on_floor {
                self.is_sliding = false;
            }
        } else {
            let move_dir = (flat_right * combined_move.x) - (flat_fwd * combined_move.y);
            let mut speed = if self.is_crouching {
                self.crouch_speed
            } else if effective_sprint && combined_move.y < -0.45 {
                self.sprint_speed
            } else {
                self.walk_speed
            };
            if self.is_aiming {
                speed *= 0.72;
            }

            let target_vx = move_dir.x * speed;
            let target_vz = move_dir.z * speed;
            let accel = if on_floor { 14.0 } else { 4.8 };
            vel.x += (target_vx - vel.x) * (dt * accel).min(1.0);
            vel.z += (target_vz - vel.z) * (dt * accel).min(1.0);
        }

        let pre_vel_y = vel.y;
        self.base_mut().set_velocity(vel);
        self.base_mut().move_and_slide();

        // Aterrizaje normal o impacto tras Dolphin Dive.
        let now_on_floor = self.base().is_on_floor();
        if self.is_diving && self.dive_airborne && now_on_floor && pre_vel_y <= 0.0 {
            self.dive_airborne = false;
            self.dive_recover_timer = 0.34;
            self.signals().landed().emit();
        } else if !self.was_on_floor && now_on_floor && pre_vel_y < -2.6 {
            self.signals().landed().emit();
        }
        self.was_on_floor = now_on_floor;

        let actual_vel = self.base().get_velocity();
        let horiz_speed = Vector2::new(actual_vel.x, actual_vel.z).length();
        if now_on_floor && self.controls_enabled && !self.is_sliding && !self.is_diving && horiz_speed > 0.65 {
            let sprinting_now = effective_sprint && combined_move.y < -0.45;
            let stride = if self.is_crouching {
                1.25
            } else if sprinting_now {
                1.48
            } else {
                1.68
            };
            self.step_distance_accum += horiz_speed * dt;
            if self.step_distance_accum >= stride {
                self.step_distance_accum = 0.0;
                let crouching_now = self.is_crouching;
                self.signals().footstep_triggered().emit(sprinting_now, crouching_now);
            }
        } else {
            self.step_distance_accum = 0.45;
        }

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

        self.apply_stance_heights(dt);
        self.apply_aim_transforms();
        self.update_visual_visibility();
        self.emit_stance_label_if_changed(effective_sprint && combined_move.y < -0.45);

        // Disparo continuo permitido incluso en slide, dive o recarga pendiente.
        if wants_fire && self.controls_enabled {
            let speed_ratio = horiz_speed / self.walk_speed.max(0.1);
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

    #[signal]
    fn footstep_triggered(is_sprinting: bool, is_crouching: bool);

    #[signal]
    fn landed();

    #[signal]
    fn aim_changed(aiming: bool);

    #[signal]
    fn grenades_changed(count: i32);

    #[signal]
    fn stance_state_changed(state_label: GString);

    #[signal]
    fn slide_triggered();

    #[signal]
    fn dive_triggered();

    #[signal]
    fn grenade_thrown();

    #[func]
    fn _on_weapon_hit_target(&mut self, damage: f32, _hit_pos: Vector3) {
        self.signals().hit_confirmed().emit(damage);
    }

    #[func]
    pub fn set_grenade_scenes(
        &mut self,
        grenade_scene: Option<Gd<PackedScene>>,
        explosion_scene: Option<Gd<PackedScene>>,
        fx_container: Option<Gd<Node3D>>,
    ) {
        self.grenade_scene = grenade_scene;
        self.explosion_scene = explosion_scene;
        self.fx_container = fx_container;
    }

    #[func]
    pub fn set_controls_enabled(&mut self, enabled: bool) {
        self.controls_enabled = enabled;
        if !enabled {
            self.move_input = Vector2::ZERO;
            self.is_firing = false;
            self.is_sprinting = false;
            self.set_aiming_internal(false);
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
        // Reduce sensibilidad al apuntar por la mira para tiros de precisión.
        let sens_scale = 1.0 - 0.34 * self.ads_blend;
        let eff_sens = self.aim_sensitivity * sens_scale;
        self.yaw -= delta.x * eff_sens;
        self.pitch = (self.pitch - delta.y * eff_sens).clamp(-1.15, 1.10);
        self.apply_aim_transforms();
    }

    #[func]
    pub fn set_firing(&mut self, firing: bool) {
        self.is_firing = firing;
    }

    /// Agacha al operador o activa un Slide si está esprintando; pulsar de nuevo durante un Slide ejecuta Slide Cancel.
    #[func]
    pub fn set_crouching(&mut self, crouching: bool) {
        if self.is_dead {
            return;
        }
        if self.is_diving {
            if !crouching {
                self.is_crouching = false;
            }
            return;
        }

        if self.is_sliding {
            // Pulsar agachar otra vez cancela inmediatamente el slide (Slide Cancel).
            self.cancel_slide(false);
            return;
        }

        if crouching {
            let vel = self.base().get_velocity();
            let horiz = Vector3::new(vel.x, 0.0, vel.z);
            let input = Input::singleton();
            let kb_sprint = input.is_action_pressed("sprint") && self.move_input.y <= -0.35;
            let wants_slide = self.base().is_on_floor()
                && self.slide_cooldown <= 0.0
                && (self.is_sprinting || kb_sprint || horiz.length() >= 5.5);

            if wants_slide {
                self.start_slide(horiz);
                return;
            }
        }
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
    pub fn set_aiming(&mut self, aiming: bool) {
        if !self.controls_enabled || self.is_dead {
            return;
        }
        if aiming && self.is_sliding {
            // Slide Cancel directo al apuntar (técnica clásica Warzone).
            self.cancel_slide(false);
        }
        if self.is_diving && aiming {
            return;
        }
        self.set_aiming_internal(aiming);
    }

    #[func]
    pub fn toggle_aiming(&mut self) {
        let next = !self.is_aiming;
        self.set_aiming(next);
    }

    #[func]
    pub fn is_player_aiming(&self) -> bool {
        self.is_aiming
    }

    /// Salta o ejecuta un Slide Cancel con impulso vertical/horizontal si está deslizando.
    #[func]
    pub fn trigger_jump(&mut self) {
        if !self.controls_enabled || self.is_dead || self.is_diving {
            return;
        }
        if self.is_sliding {
            self.cancel_slide(true);
            return;
        }
        if self.base().is_on_floor() {
            self.is_crouching = false;
            let mut vel = self.base().get_velocity();
            vel.y = self.jump_velocity;
            self.base_mut().set_velocity(vel);
        }
    }

    /// Lanzamiento en plancha ("Dolphin Dive") hacia adelante para cruzar fuego enemigo o cubrirse.
    #[func]
    pub fn trigger_dive(&mut self) {
        if !self.controls_enabled || self.is_dead || self.is_diving || self.dive_cooldown > 0.0 {
            return;
        }
        if !self.base().is_on_floor() && !self.is_sliding {
            return;
        }

        self.is_sliding = false;
        self.is_sprinting = false;
        self.set_aiming_internal(false);

        let xform = self.base().get_global_transform();
        let forward = -xform.basis.col_c();
        let right = xform.basis.col_a();
        let flat_fwd = Vector3::new(forward.x, 0.0, forward.z).normalized_or_zero();
        let flat_right = Vector3::new(right.x, 0.0, right.z).normalized_or_zero();

        let move_vec = (flat_right * self.move_input.x) - (flat_fwd * self.move_input.y);
        self.dive_dir = if move_vec.length_squared() > 0.04 {
            move_vec.normalized()
        } else {
            flat_fwd
        };

        self.is_diving = true;
        self.dive_airborne = true;
        self.dive_recover_timer = 0.0;
        self.dive_cooldown = 0.85;
        self.is_crouching = true;

        let mut vel = self.base().get_velocity();
        vel.x = self.dive_dir.x * 8.9;
        vel.z = self.dive_dir.z * 8.9;
        vel.y = 3.7;
        self.base_mut().set_velocity(vel);
        self.signals().dive_triggered().emit();
    }

    /// Lanza una granada de fragmentación siguiendo la orientación actual de la mira.
    #[func]
    pub fn trigger_grenade(&mut self) {
        if !self.controls_enabled || self.is_dead || self.grenade_cooldown > 0.0 || self.grenades_left <= 0 {
            return;
        }
        let Some(grenade_scene) = self.grenade_scene.clone() else {
            return;
        };
        let Some(mut container) = self.resolve_fx_parent() else {
            return;
        };
        let Some(cam) = self.camera.clone() else {
            return;
        };
        let Some(instance) = grenade_scene.instantiate() else {
            return;
        };
        let Ok(mut model_root) = instance.try_cast::<Node3D>() else {
            return;
        };

        let cam_xform = cam.get_global_transform();
        let aim_dir = (-cam_xform.basis.col_c()).normalized_or_zero();
        let start_pos = self.base().get_global_position()
            + Vector3::new(0.0, if self.is_crouching { 1.0 } else { 1.42 }, 0.0)
            + aim_dir * 0.65;
        let throw_vel = aim_dir * 17.6 + Vector3::new(0.0, 4.3, 0.0);

        let mut grenade_node = FragGrenade::new_alloc();
        grenade_node.set_name("FragGrenadeProjectile");
        grenade_node.add_child(&model_root);
        container.add_child(&grenade_node);

        let spin_anim = find_animation_player_recursive(&model_root);
        let thrower_rid = Some(self.base().get_rid());
        grenade_node.bind_mut().launch(
            start_pos,
            throw_vel,
            thrower_rid,
            self.explosion_scene.clone(),
            Some(container),
            spin_anim,
        );

        self.grenades_left -= 1;
        self.grenade_cooldown = 0.65;
        let left = self.grenades_left;
        self.signals().grenades_changed().emit(left);
        self.signals().grenade_thrown().emit();
    }

    #[func]
    pub fn get_grenades_left(&self) -> i32 {
        self.grenades_left
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
                Vector3::new(0.0, 0.08, -0.22)
            } else {
                Vector3::new(0.54, 0.14, 0.0)
            };
            arm.set_position(pos);
        }
        self.apply_stance_heights(0.016);
        self.update_visual_visibility();
    }

    #[func]
    pub fn get_weapon_rig(&self) -> Option<Gd<WeaponRig>> {
        self.weapon_rig.clone()
    }

    #[func]
    pub fn get_health(&self) -> f32 {
        self.health
    }

    /// Reinicia posición, vida, rotación inicial mirando al enemigo, granadas y munición al reaparecer.
    #[func]
    pub fn reset_for_round(&mut self, spawn_pos: Vector3, face_target: Vector3) {
        self.is_dead = false;
        self.health = self.max_health;
        self.is_crouching = false;
        self.is_sliding = false;
        self.is_diving = false;
        self.dive_airborne = false;
        self.slide_timer = 0.0;
        self.slide_cooldown = 0.0;
        self.dive_cooldown = 0.0;
        self.grenade_cooldown = 0.0;
        self.grenades_left = self.max_grenades;
        self.crouch_blend = 0.0;
        self.ads_blend = 0.0;
        self.is_firing = false;
        self.is_sprinting = false;
        self.move_input = Vector2::ZERO;
        self.recoil_recover_pitch = 0.0;
        self.recoil_recover_yaw = 0.0;
        self.was_on_floor = true;
        self.step_distance_accum = 0.45;
        self.set_aiming_internal(false);

        if let Some(mut col) = self.collision_shape.clone() {
            col.set_disabled(false);
        }
        if let Some(mut w_mount) = self.weapon_mount.clone() {
            w_mount.set_visible(true);
        }

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
        self.apply_stance_heights(0.016);
        self.apply_aim_transforms();
        self.update_visual_visibility();

        if let Some(mut rig) = self.weapon_rig.clone() {
            rig.bind_mut().reset_ammo();
        }

        let hp = self.health;
        let max_hp = self.max_health;
        let g_left = self.grenades_left;
        self.signals().health_changed().emit(hp, max_hp);
        self.signals().grenades_changed().emit(g_left);
        self.last_stance_label = "DE PIE".to_string();
        let init_stance = GString::from("DE PIE");
        self.signals().stance_state_changed().emit(&init_stance);
    }

    /// Recibe impacto balístico o de metralla.
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
            self.is_sliding = false;
            self.is_diving = false;
            self.controls_enabled = false;
            self.set_aiming_internal(false);
            if let Some(mut col) = self.collision_shape.clone() {
                col.set_disabled(true);
            }
            if let Some(mut vis) = self.visual_pivot.clone() {
                vis.set_visible(false);
            }
            if let Some(mut w_mount) = self.weapon_mount.clone() {
                w_mount.set_visible(false);
            }
            self.signals().died().emit();
        }
    }

    fn start_slide(&mut self, current_horiz: Vector3) {
        let xform = self.base().get_global_transform();
        let forward = -xform.basis.col_c();
        let flat_fwd = Vector3::new(forward.x, 0.0, forward.z).normalized_or_zero();

        self.slide_dir = if current_horiz.length_squared() > 0.25 {
            current_horiz.normalized()
        } else {
            flat_fwd
        };
        self.is_sliding = true;
        self.is_crouching = true;
        self.slide_timer = 0.0;
        self.slide_cooldown = 0.28;
        self.set_aiming_internal(false);

        let mut vel = self.base().get_velocity();
        vel.x = self.slide_dir.x * 9.8;
        vel.z = self.slide_dir.z * 9.8;
        self.base_mut().set_velocity(vel);
        self.signals().slide_triggered().emit();
    }

    fn cancel_slide(&mut self, jump_hop: bool) {
        if !self.is_sliding {
            return;
        }
        self.is_sliding = false;
        self.is_crouching = false;
        self.slide_cooldown = 0.14;
        self.crouch_blend = self.crouch_blend.min(0.40);

        let mut vel = self.base().get_velocity();
        vel.x = self.slide_dir.x * 6.8;
        vel.z = self.slide_dir.z * 6.8;
        if jump_hop && self.base().is_on_floor() {
            vel.y = self.jump_velocity * 0.92;
        }
        self.base_mut().set_velocity(vel);
    }

    fn set_aiming_internal(&mut self, aiming: bool) {
        if self.is_aiming == aiming {
            return;
        }
        self.is_aiming = aiming;
        if let Some(mut rig) = self.weapon_rig.clone() {
            rig.bind_mut().set_aiming(aiming);
        }
        self.signals().aim_changed().emit(aiming);
    }

    fn emit_stance_label_if_changed(&mut self, sprinting_now: bool) {
        let next = if self.is_diving {
            "LANZÁNDOSE"
        } else if self.is_sliding {
            "DESLIZANDO"
        } else if self.is_aiming {
            if self.is_crouching {
                "APUNTANDO (AG.)"
            } else {
                "APUNTANDO"
            }
        } else if self.is_crouching {
            "AGACHADO"
        } else if sprinting_now {
            "SPRINT"
        } else {
            "DE PIE"
        };

        if self.last_stance_label != next {
            self.last_stance_label = next.to_string();
            let gs = GString::from(next);
            self.signals().stance_state_changed().emit(&gs);
        }
    }

    /// Oculta la malla de cápsula en 1ª persona o si el brazo de cámara choca con un muro cercano.
    fn update_visual_visibility(&mut self) {
        let mut show_capsule = !self.is_dead && !self.first_person_mode;
        if show_capsule {
            if let (Some(cam), Some(pivot)) = (self.camera.clone(), self.camera_pivot.clone()) {
                let cam_dist = (cam.get_global_position() - pivot.get_global_position()).length();
                if cam_dist < 0.88 {
                    show_capsule = false;
                }
            }
        }
        if let Some(mut vis) = self.visual_pivot.clone() {
            vis.set_visible(show_capsule);
        }
    }

    fn apply_stance_heights(&mut self, dt: f32) {
        let blend = self.crouch_blend;
        let extra_drop = if self.is_sliding || self.is_diving { 0.16 } else { 0.0 };
        let col_height = (1.80 - 0.62 * blend - extra_drop).max(0.88);

        if let Some(mut col) = self.collision_shape.clone() {
            col.set_position(Vector3::new(0.0, col_height * 0.5, 0.0));
            if let Some(shape) = col.get_shape() {
                if let Ok(mut capsule) = shape.try_cast::<CapsuleShape3D>() {
                    capsule.set_height(col_height);
                }
            }
        }

        if let Some(mut vis) = self.visual_pivot.clone() {
            let y_scale = 1.0 - 0.34 * blend;
            vis.set_scale(Vector3::new(1.0, y_scale, 1.0));
            let body_pitch = if self.is_diving {
                if self.dive_airborne { -0.95 } else { -0.68 }
            } else if self.is_sliding {
                0.42
            } else {
                0.0
            };
            vis.set_rotation(Vector3::new(body_pitch, 0.0, 0.0));
        }

        if let Some(mut cam_piv) = self.camera_pivot.clone() {
            let cam_y = 1.48 - 0.50 * blend - (if self.is_sliding || self.is_diving { 0.14 } else { 0.0 });
            cam_piv.set_position(Vector3::new(0.0, cam_y, 0.0));
        }

        if let Some(mut w_mount) = self.weapon_mount.clone() {
            let drop_y = 0.50 * blend + (if self.is_sliding || self.is_diving { 0.14 } else { 0.0 });
            let (hip_pos, ads_pos) = if self.first_person_mode {
                (
                    Vector3::new(0.23, 1.34 - drop_y, -0.46),
                    Vector3::new(0.0, 1.495 - drop_y, -0.34),
                )
            } else {
                (
                    Vector3::new(0.31, 1.24 - drop_y * 0.84, -0.27),
                    Vector3::new(0.22, 1.32 - drop_y * 0.84, -0.33),
                )
            };
            let pos = hip_pos + (ads_pos - hip_pos) * self.ads_blend;
            w_mount.set_position(pos);
        }

        // Zoom dinámico de campo de visión (FOV) al apuntar ADS o esprintar/deslizar.
        if let Some(mut cam) = self.camera.clone() {
            let speed_boost = if self.is_sliding || self.is_diving { 5.5 } else { 0.0 };
            let target_fov = (72.0 + speed_boost) * (1.0 - self.ads_blend) + 52.0 * self.ads_blend;
            let current_fov = cam.get_fov();
            cam.set_fov(current_fov + (target_fov - current_fov) * (dt * 12.0).min(1.0));
        }
    }

    fn apply_aim_transforms(&mut self) {
        let yaw = self.yaw;
        self.base_mut().set_rotation(Vector3::new(0.0, yaw, 0.0));
        let roll_tilt = if self.is_sliding { -0.09 } else { 0.0 };
        if let Some(mut cam_piv) = self.camera_pivot.clone() {
            cam_piv.set_rotation(Vector3::new(self.pitch, 0.0, roll_tilt));
        }
        if let Some(mut w_mount) = self.weapon_mount.clone() {
            let weapon_roll = if self.is_sliding { -0.26 } else { 0.0 };
            w_mount.set_rotation(Vector3::new(self.pitch, 0.0, weapon_roll));
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
        let spread = calculate_tactical_spread_deg(
            0.75,
            speed_ratio,
            self.is_crouching || self.is_sliding || self.is_diving,
            self.is_aiming && self.ads_blend > 0.45,
            heat,
        );
        let fired = rig.bind_mut().try_fire(aim_origin, aim_dir, spread);
        let hit_dmg = rig.bind_mut().consume_last_hit_damage();

        if fired {
            let (kick_pitch, kick_yaw) = rig.bind_mut().consume_recoil_kick();
            self.pitch = (self.pitch + kick_pitch).clamp(-1.15, 1.10);
            self.yaw += kick_yaw;
            self.recoil_recover_pitch += kick_pitch * 0.72;
            self.recoil_recover_yaw += kick_yaw * 0.55;
            self.apply_aim_transforms();
        }
        if let Some(dmg) = hit_dmg {
            self.signals().hit_confirmed().emit(dmg);
        }
    }

    fn resolve_fx_parent(&self) -> Option<Gd<Node3D>> {
        if let Some(container) = self.fx_container.clone() {
            return Some(container);
        }
        self.base()
            .get_tree_or_null()
            .and_then(|tree| tree.get_current_scene())
            .and_then(|scene| scene.try_cast::<Node3D>().ok())
    }
}
