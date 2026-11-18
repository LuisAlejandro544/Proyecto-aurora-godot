//! Lógica del rifle de asalto 7.62 (`WeaponRig`) y entidades físicas/visuales de proyectil y casquillo.
//!
//! Gestiona munición (30/120), cadencia de disparo (~570 RPM), retroceso acumulativo,
//! modo de apuntado ADS con recarga en mira (`aim_idle`, `aim_reload`), animaciones del
//! modelo GLB (`idle`, `fire`, `reload`), fogonazo (`flash_burst`), trazadoras de bala en vuelo
//! (`TracerProjectile`), expulsión de casquillos (`EjectedCasing`) y raycast 3D de doble etapa.

use godot::classes::{
    AnimationPlayer, CollisionObject3D, INode3D, Node3D, PackedScene, PhysicsRayQueryParameters3D,
};
use godot::prelude::*;

use crate::combat_math::{
    calculate_damage_with_falloff, compute_spread_angles_rad,
};

/// Proyectil trazador en vuelo que avanza desde la boca del cañón hasta el punto de impacto.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct TracerProjectile {
    base: Base<Node3D>,
    velocity: Vector3,
    #[init(val = 1.0)]
    max_distance: f32,
    distance_traveled: f32,
    #[init(val = 0.5)]
    ttl: f64,
}

#[godot_api]
impl INode3D for TracerProjectile {
    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        let step = self.velocity * dt;
        let step_len = step.length();

        let current_pos = self.base().get_global_position();
        self.base_mut().set_global_position(current_pos + step);

        self.distance_traveled += step_len;
        self.ttl -= delta;

        if self.distance_traveled >= self.max_distance || self.ttl <= 0.0 {
            self.base_mut().queue_free();
        }
    }
}

#[godot_api]
impl TracerProjectile {
    /// Inicializa la trayectoria del proyectil desde la boca del rifle hacia el impacto.
    #[func]
    pub fn launch(&mut self, start_pos: Vector3, direction: Vector3, speed: f32, max_dist: f32) {
        let dir = if direction.length_squared() > 0.0001 {
            direction.normalized()
        } else {
            Vector3::FORWARD
        };
        self.base_mut().set_global_position(start_pos);
        self.base_mut().look_at_ex(start_pos + dir).up(Vector3::UP).done();
        self.velocity = dir * speed.max(20.0);
        self.max_distance = max_dist.max(0.5);
        self.distance_traveled = 0.0;
        self.ttl = (self.max_distance / speed.max(20.0)) as f64 + 0.06;
    }
}

/// Casquillo de latón expulsado que reproduce la animación `casing_eject` y se libera automáticamente.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct EjectedCasing {
    base: Base<Node3D>,
    #[init(val = 1.65)]
    ttl: f64,
}

#[godot_api]
impl INode3D for EjectedCasing {
    fn process(&mut self, delta: f64) {
        self.ttl -= delta;
        if self.ttl <= 0.0 {
            self.base_mut().queue_free();
        }
    }
}

#[godot_api]
impl EjectedCasing {
    /// Reproduce la animación de expulsión y caída del archivo `shell_casing_eject.glb`.
    #[func]
    pub fn play_eject_animation(&mut self, anim_player: Option<Gd<AnimationPlayer>>) {
        if let Some(mut anim) = anim_player {
            play_matching_animation(&mut anim, "casing_eject");
        }
        self.ttl = 1.65;
    }
}

/// Reproduce una animación buscando coincidencia exacta o por sufijo en el `AnimationPlayer`.
fn play_matching_animation(anim: &mut Gd<AnimationPlayer>, target: &str) -> bool {
    if anim.has_animation(target) {
        anim.stop();
        anim.play_ex().name(target).done();
        return true;
    }
    let list = anim.get_animation_list();
    for idx in 0..list.len() {
        let candidate = list.get(idx).unwrap_or_default().to_string();
        if candidate.ends_with(target) {
            anim.stop();
            anim.play_ex().name(&candidate).done();
            return true;
        }
    }
    false
}

/// Intenta reproducir una animación preferida (ej. `aim_reload`) con respaldo seguro (ej. `reload`).
fn play_animation_with_fallback(anim: &mut Gd<AnimationPlayer>, preferred: &str, fallback: &str) {
    if !play_matching_animation(anim, preferred) && preferred != fallback {
        play_matching_animation(anim, fallback);
    }
}

/// Controlador modular del rifle clásico 7.62 compartido por el jugador y la IA.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct WeaponRig {
    base: Base<Node3D>,

    #[init(val = 30)]
    mag_capacity: i32,
    #[init(val = 30)]
    ammo_in_mag: i32,
    #[init(val = 120)]
    reserve_ammo: i32,
    #[init(val = 24.0)]
    base_damage: f32,
    #[init(val = 0.105)]
    fire_interval: f64,
    #[init(val = 2.30)]
    reload_duration: f64,

    fire_cooldown: f64,
    reload_timer: f64,
    is_reloading: bool,
    is_aiming: bool,
    flash_timer: f64,
    recoil_heat: f32,
    pending_recoil_pitch: f32,
    pending_recoil_yaw: f32,
    shot_counter: u32,
    last_hit_damage: Option<f32>,

    anim_player: Option<Gd<AnimationPlayer>>,
    muzzle_point: Option<Gd<Node3D>>,
    eject_point: Option<Gd<Node3D>>,
    muzzle_flash_node: Option<Gd<Node3D>>,
    muzzle_flash_anim: Option<Gd<AnimationPlayer>>,
    bullet_scene: Option<Gd<PackedScene>>,
    casing_scene: Option<Gd<PackedScene>>,
    fx_container: Option<Gd<Node3D>>,
    shooter_rid: Option<Rid>,
}

#[godot_api]
impl INode3D for WeaponRig {
    fn ready(&mut self) {
        if let Some(mut flash) = self.muzzle_flash_node.clone() {
            flash.set_visible(false);
        }
    }

    fn process(&mut self, delta: f64) {
        if self.fire_cooldown > 0.0 {
            self.fire_cooldown = (self.fire_cooldown - delta).max(0.0);
        }

        // Enfriamiento progresivo de la dispersión por ráfaga continua.
        self.recoil_heat = (self.recoil_heat - (delta as f32) * 1.45).max(0.0);

        if self.flash_timer > 0.0 {
            self.flash_timer -= delta;
            if self.flash_timer <= 0.0 {
                if let Some(mut flash) = self.muzzle_flash_node.clone() {
                    flash.set_visible(false);
                }
            }
        }

        if self.is_reloading {
            self.reload_timer -= delta;
            if self.reload_timer <= 0.0 {
                self.finish_reload();
            }
        } else if let Some(mut anim) = self.anim_player.clone() {
            if !anim.is_playing() {
                let idle_name = if self.is_aiming { "aim_idle" } else { "idle" };
                play_animation_with_fallback(&mut anim, idle_name, "idle");
            }
        }
    }
}

#[godot_api]
impl WeaponRig {
    /// Notifica a la interfaz los cambios en cargador y reserva.
    #[signal]
    fn ammo_changed(in_mag: i32, reserve: i32);

    /// Se emite al iniciar una recarga táctica con su duración en segundos.
    #[signal]
    fn reload_started(duration: f64);

    /// Se emite al finalizar la recarga y quedar el arma lista para disparar.
    #[signal]
    fn reload_finished();

    /// Se emite cada vez que sale un proyectil del rifle.
    #[signal]
    fn weapon_fired();

    /// Se emite cuando un disparo impacta a un objetivo dañable.
    #[signal]
    fn hit_target(damage: f32, hit_pos: Vector3);

    /// Conecta las referencias internas del modelo GLB del rifle y del fogonazo.
    #[func]
    pub fn configure_parts(
        &mut self,
        anim_player: Option<Gd<AnimationPlayer>>,
        muzzle_point: Option<Gd<Node3D>>,
        eject_point: Option<Gd<Node3D>>,
        muzzle_flash_node: Option<Gd<Node3D>>,
        muzzle_flash_anim: Option<Gd<AnimationPlayer>>,
    ) {
        self.anim_player = anim_player;
        self.muzzle_point = muzzle_point;
        self.eject_point = eject_point;
        self.muzzle_flash_node = muzzle_flash_node;
        self.muzzle_flash_anim = muzzle_flash_anim;

        if let Some(mut flash) = self.muzzle_flash_node.clone() {
            flash.set_visible(false);
        }
        if let Some(mut anim) = self.anim_player.clone() {
            let idle_name = if self.is_aiming { "aim_idle" } else { "idle" };
            play_animation_with_fallback(&mut anim, idle_name, "idle");
        }
    }

    /// Asigna los `PackedScene` dinámicos de la bala y el casquillo expulsado.
    #[func]
    pub fn set_fx_scenes(
        &mut self,
        bullet_scene: Option<Gd<PackedScene>>,
        casing_scene: Option<Gd<PackedScene>>,
    ) {
        self.bullet_scene = bullet_scene;
        self.casing_scene = casing_scene;
    }

    /// Establece el nodo raíz de escena donde viajan trazadoras y casquillos sin pegarse al arma.
    #[func]
    pub fn set_fx_container(&mut self, container: Option<Gd<Node3D>>) {
        self.fx_container = container;
    }

    /// Excluye el `RID` del tirador en los raycasts balísticos para evitar auto-impactos.
    #[func]
    pub fn set_shooter_body(&mut self, body: Option<Gd<CollisionObject3D>>) {
        self.shooter_rid = body.map(|b| b.get_rid());
    }

    /// Ajusta el daño base (útil para balancear dificultad de la IA).
    #[func]
    pub fn set_base_damage(&mut self, dmg: f32) {
        self.base_damage = dmg.max(1.0);
    }

    /// Activa o desactiva la postura de apuntado por mira (ADS), adaptando incluso recargas en curso.
    #[func]
    pub fn set_aiming(&mut self, aiming: bool) {
        if self.is_aiming == aiming {
            return;
        }
        self.is_aiming = aiming;
        let Some(mut anim) = self.anim_player.clone() else {
            return;
        };

        if self.is_reloading {
            // Cambia suavemente entre `reload` y `aim_reload` conservando el tiempo transcurrido.
            let elapsed = (self.reload_duration - self.reload_timer).max(0.0);
            let target = if aiming { "aim_reload" } else { "reload" };
            play_animation_with_fallback(&mut anim, target, "reload");
            anim.call("seek", &[elapsed.to_variant(), true.to_variant()]);
        } else {
            let idle_name = if aiming { "aim_idle" } else { "idle" };
            play_animation_with_fallback(&mut anim, idle_name, "idle");
        }
    }

    #[func]
    pub fn is_weapon_aiming(&self) -> bool {
        self.is_aiming
    }

    /// Restablece la munición y detiene recargas al comenzar una nueva ronda.
    #[func]
    pub fn reset_ammo(&mut self) {
        self.ammo_in_mag = self.mag_capacity;
        self.reserve_ammo = 120;
        self.is_reloading = false;
        self.is_aiming = false;
        self.reload_timer = 0.0;
        self.fire_cooldown = 0.0;
        self.recoil_heat = 0.0;
        self.pending_recoil_pitch = 0.0;
        self.pending_recoil_yaw = 0.0;
        self.last_hit_damage = None;
        if let Some(mut flash) = self.muzzle_flash_node.clone() {
            flash.set_visible(false);
        }
        if let Some(mut anim) = self.anim_player.clone() {
            play_animation_with_fallback(&mut anim, "idle", "idle");
        }
        let in_mag = self.ammo_in_mag;
        let reserve = self.reserve_ammo;
        self.signals().ammo_changed().emit(in_mag, reserve);
    }

    #[func]
    pub fn get_ammo_in_mag(&self) -> i32 {
        self.ammo_in_mag
    }

    #[func]
    pub fn get_reserve_ammo(&self) -> i32 {
        self.reserve_ammo
    }

    #[func]
    pub fn is_weapon_reloading(&self) -> bool {
        self.is_reloading
    }

    #[func]
    pub fn get_recoil_heat(&self) -> f32 {
        self.recoil_heat
    }

    #[func]
    pub fn get_reload_progress(&self) -> f32 {
        if !self.is_reloading || self.reload_duration <= 0.0 {
            return 0.0;
        }
        (1.0 - (self.reload_timer / self.reload_duration) as f32).clamp(0.0, 1.0)
    }

    /// Entrega y consume el retroceso acumulado del último disparo para elevar la cámara del jugador.
    pub fn consume_recoil_kick(&mut self) -> (f32, f32) {
        let out = (self.pending_recoil_pitch, self.pending_recoil_yaw);
        self.pending_recoil_pitch = 0.0;
        self.pending_recoil_yaw = 0.0;
        out
    }

    /// Consume el daño del último impacto confirmado sin re-entrar por señal en el dueño.
    pub fn consume_last_hit_damage(&mut self) -> Option<f32> {
        self.last_hit_damage.take()
    }

    /// Inicia una recarga si el cargador no está lleno y queda munición de reserva.
    /// Permite recargar tanto desde cadera (`reload`) como apuntando (`aim_reload`).
    #[func]
    pub fn try_reload(&mut self) -> bool {
        if self.is_reloading || self.ammo_in_mag >= self.mag_capacity || self.reserve_ammo <= 0 {
            return false;
        }
        self.is_reloading = true;
        self.reload_timer = self.reload_duration;
        if let Some(mut anim) = self.anim_player.clone() {
            let anim_name = if self.is_aiming { "aim_reload" } else { "reload" };
            play_animation_with_fallback(&mut anim, anim_name, "reload");
        }
        let duration = self.reload_duration;
        self.signals().reload_started().emit(duration);
        true
    }

    fn finish_reload(&mut self) {
        self.is_reloading = false;
        self.reload_timer = 0.0;
        let needed = (self.mag_capacity - self.ammo_in_mag).max(0);
        let transferred = needed.min(self.reserve_ammo);
        self.ammo_in_mag += transferred;
        self.reserve_ammo -= transferred;

        if let Some(mut anim) = self.anim_player.clone() {
            let idle_name = if self.is_aiming { "aim_idle" } else { "idle" };
            play_animation_with_fallback(&mut anim, idle_name, "idle");
        }
        let in_mag = self.ammo_in_mag;
        let reserve = self.reserve_ammo;
        self.signals().ammo_changed().emit(in_mag, reserve);
        self.signals().reload_finished().emit();
    }

    /// Intenta disparar un proyectil desde `aim_origin` hacia `aim_dir` con dispersión angular.
    /// Retorna `true` si el arma disparó efectivamente en este frame.
    #[func]
    pub fn try_fire(&mut self, aim_origin: Vector3, aim_dir: Vector3, spread_deg: f32) -> bool {
        if self.is_reloading || self.fire_cooldown > 0.0 {
            return false;
        }
        if self.ammo_in_mag <= 0 {
            self.try_reload();
            return false;
        }

        self.ammo_in_mag -= 1;
        self.fire_cooldown = self.fire_interval;
        self.shot_counter = self.shot_counter.wrapping_add(1);
        self.last_hit_damage = None;
        self.recoil_heat = (self.recoil_heat + 0.16).min(1.0);

        // Retroceso físico ascendente + leve deriva lateral alternante (más controlado en ADS).
        let ads_recoil_scale = if self.is_aiming { 0.68 } else { 1.0 };
        let side_sign = if self.shot_counter % 2 == 0 { 1.0 } else { -1.0 };
        self.pending_recoil_pitch += (0.022 + self.recoil_heat * 0.012) * ads_recoil_scale;
        self.pending_recoil_yaw += side_sign * (0.005 + self.recoil_heat * 0.006) * ads_recoil_scale;

        // Animación del rifle y fogonazo.
        if let Some(mut anim) = self.anim_player.clone() {
            play_matching_animation(&mut anim, "fire");
        }
        self.trigger_muzzle_flash();
        self.spawn_ejected_casing();

        // Cálculo de dirección con dispersión balística.
        let dir_norm = if aim_dir.length_squared() > 0.0001 {
            aim_dir.normalized()
        } else {
            Vector3::FORWARD
        };
        let (yaw_off, pitch_off) = compute_spread_angles_rad(self.shot_counter, spread_deg);
        let spread_dir = apply_angular_spread(dir_norm, yaw_off, pitch_off);

        self.perform_ballistic_raycast(aim_origin, spread_dir);

        let in_mag = self.ammo_in_mag;
        let reserve = self.reserve_ammo;
        self.signals().ammo_changed().emit(in_mag, reserve);
        self.signals().weapon_fired().emit();

        if self.ammo_in_mag == 0 && self.reserve_ammo > 0 {
            self.try_reload();
        }
        true
    }

    fn trigger_muzzle_flash(&mut self) {
        if let Some(mut flash) = self.muzzle_flash_node.clone() {
            flash.set_visible(true);
            // En glTF el fogonazo apunta por +X, rotarlo alrededor de X varía las aletas en cada tiro.
            let roll = ((self.shot_counter % 7) as f32) * 0.89;
            flash.set_rotation(Vector3::new(roll, 0.0, 0.0));
        }
        if let Some(mut flash_anim) = self.muzzle_flash_anim.clone() {
            play_matching_animation(&mut flash_anim, "flash_burst");
        }
        self.flash_timer = 0.075;
    }

    fn spawn_ejected_casing(&mut self) {
        let Some(casing_scene) = self.casing_scene.clone() else {
            return;
        };
        let Some(eject) = self.eject_point.clone() else {
            return;
        };
        let Some(mut container) = self.resolve_fx_parent() else {
            return;
        };

        let Some(instance) = casing_scene.instantiate() else {
            return;
        };
        let Ok(mut model_root) = instance.try_cast::<Node3D>() else {
            return;
        };

        // `shell_casing_eject.glb` expulsa hacia glTF +Z; rotando +90° en Y el eje +Z apunta hacia +X (derecha).
        model_root.set_rotation(Vector3::new(0.0, std::f32::consts::FRAC_PI_2, 0.0));

        let mut wrapper = EjectedCasing::new_alloc();
        wrapper.set_name("EjectedCasingFX");
        wrapper.add_child(&model_root);
        container.add_child(&wrapper);

        let mut eject_xform = eject.get_global_transform();
        // Mantiene la orientación horizontal del rifle al expulsar para evitar inclinaciones raras.
        let forward = -eject_xform.basis.col_c();
        let flat_fwd = Vector3::new(forward.x, 0.0, forward.z);
        if flat_fwd.length_squared() > 0.001 {
            wrapper.set_global_position(eject_xform.origin);
            wrapper.look_at_ex(eject_xform.origin + flat_fwd.normalized()).up(Vector3::UP).done();
        } else {
            eject_xform.basis = Basis::IDENTITY;
            wrapper.set_global_transform(eject_xform);
        }

        let anim = model_root.try_get_node_as::<AnimationPlayer>("AnimationPlayer");
        wrapper.bind_mut().play_eject_animation(anim);
    }

    fn spawn_tracer(&mut self, start_pos: Vector3, hit_pos: Vector3) {
        let Some(bullet_scene) = self.bullet_scene.clone() else {
            return;
        };
        let Some(mut container) = self.resolve_fx_parent() else {
            return;
        };
        let Some(instance) = bullet_scene.instantiate() else {
            return;
        };
        let Ok(mut bullet_root) = instance.try_cast::<Node3D>() else {
            return;
        };

        // `bullet_projectile.glb` apunta en glTF +X; rotado +90° en Y apunta al frente de Godot (-Z).
        bullet_root.set_rotation(Vector3::new(0.0, std::f32::consts::FRAC_PI_2, 0.0));
        bullet_root.set_scale(Vector3::new(2.2, 2.2, 2.2));

        let delta = hit_pos - start_pos;
        let dist = delta.length().max(0.5);
        let dir = delta / dist;

        let mut tracer = TracerProjectile::new_alloc();
        tracer.set_name("BulletTracerFX");
        tracer.add_child(&bullet_root);
        container.add_child(&tracer);
        tracer.bind_mut().launch(start_pos, dir, 95.0, dist);
    }

    fn perform_ballistic_raycast(&mut self, aim_origin: Vector3, spread_dir: Vector3) {
        let Some(world) = self.base().get_world_3d() else {
            return;
        };
        let Some(mut space) = world.get_direct_space_state() else {
            return;
        };

        let max_range = 120.0_f32;
        let mut exclude = Array::<Rid>::new();
        if let Some(rid) = self.shooter_rid {
            exclude.push(rid);
        }

        // Etapa 1: rayo desde la cámara/mira para hallar el punto exacto donde apunta la retícula.
        let mut aim_query = PhysicsRayQueryParameters3D::new_gd();
        aim_query.set_from(aim_origin);
        aim_query.set_to(aim_origin + spread_dir * max_range);
        aim_query.set_exclude(&exclude);
        aim_query.set_collide_with_bodies(true);
        aim_query.set_collide_with_areas(false);

        let aim_hit = space.intersect_ray(&aim_query);
        let mut target_point = aim_origin + spread_dir * max_range;
        if let Some(pos_var) = aim_hit.get("position") {
            if let Ok(pos) = pos_var.try_to::<Vector3>() {
                target_point = pos;
            }
        }

        // Etapa 2: rayo físico real desde la boca del cañón hacia el punto de mira (evita atravesar muros cercanos).
        let muzzle_pos = if let Some(muzzle) = self.muzzle_point.clone() {
            muzzle.get_global_position()
        } else {
            self.base().get_global_position()
        };

        let muzzle_vec = target_point - muzzle_pos;
        let muzzle_dir = if muzzle_vec.length_squared() > 0.0001 {
            muzzle_vec.normalized()
        } else {
            spread_dir
        };

        let mut muzzle_query = PhysicsRayQueryParameters3D::new_gd();
        muzzle_query.set_from(muzzle_pos);
        muzzle_query.set_to(muzzle_pos + muzzle_dir * max_range);
        muzzle_query.set_exclude(&exclude);
        muzzle_query.set_collide_with_bodies(true);
        muzzle_query.set_collide_with_areas(false);

        let hit = space.intersect_ray(&muzzle_query);
        let mut final_hit_pos = muzzle_pos + muzzle_dir * max_range;

        if !hit.is_empty() {
            if let Some(pos_var) = hit.get("position") {
                if let Ok(pos) = pos_var.try_to::<Vector3>() {
                    final_hit_pos = pos;
                }
            }
            if let Some(col_var) = hit.get("collider") {
                if let Ok(mut collider) = col_var.try_to::<Gd<Object>>() {
                    if collider.has_method("apply_bullet_damage") {
                        let dist = (final_hit_pos - muzzle_pos).length();
                        let dmg = calculate_damage_with_falloff(self.base_damage, dist);
                        collider.call(
                            "apply_bullet_damage",
                            &[dmg.to_variant(), muzzle_pos.to_variant()],
                        );
                        self.last_hit_damage = Some(dmg);
                        self.signals().hit_target().emit(dmg, final_hit_pos);
                    }
                }
            }
        }

        self.spawn_tracer(muzzle_pos, final_hit_pos);
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

/// Aplica rotación horizontal (yaw) y vertical (pitch) de dispersión sobre un vector unitario.
fn apply_angular_spread(forward: Vector3, yaw_rad: f32, pitch_rad: f32) -> Vector3 {
    let mut right = forward.cross(Vector3::UP);
    if right.length_squared() < 0.0001 {
        right = Vector3::RIGHT;
    } else {
        right = right.normalized();
    }
    let up = right.cross(forward).normalized();
    (forward + right * yaw_rad.tan() + up * pitch_rad.tan()).normalized()
}
