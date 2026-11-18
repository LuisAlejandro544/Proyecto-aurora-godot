//! Inteligencia Artificial táctica local (`EnemyAiCapsule`) con comportamiento humano.
//!
//! La cápsula enemiga utiliza `NavigationAgent3D`, evaluación real de línea de visión
//! con raycasts 3D contra las coberturas de la ciudad destruida, tiempo de reacción humano,
//! giro angular suave, ráfagas controladas de AK-47, agachado tras barreras al recargar
//! o recibir fuego, y flanqueo si el jugador permanece escondido demasiado tiempo.

use godot::classes::{
    CapsuleShape3D, CharacterBody3D, CollisionObject3D, CollisionShape3D, ICharacterBody3D,
    NavigationAgent3D, Node3D, PhysicsRayQueryParameters3D,
};
use godot::prelude::*;

use crate::combat_math::{
    calculate_effective_spread_deg, compute_smooth_yaw_step, score_cover_candidate,
    select_burst_length, should_flank,
};
use crate::weapon_rig::WeaponRig;

/// Estados tácticos de la máquina de estados finitos de la IA.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum AiTacticalState {
    #[default]
    PatrolAdvance,
    EngageBurst,
    SeekCover,
    ReloadInCover,
    Flank,
}

impl AiTacticalState {
    fn label(self) -> &'static str {
        match self {
            Self::PatrolAdvance => "Avanzando",
            Self::EngageBurst => "En Combate",
            Self::SeekCover => "Buscando Cobertura",
            Self::ReloadInCover => "Recargando Cubierto",
            Self::Flank => "Flanqueando",
        }
    }
}

/// Controlador de la cápsula enemiga inteligente equipada con el rifle clásico 7.62.
#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct EnemyAiCapsule {
    base: Base<CharacterBody3D>,

    #[init(val = 100.0)]
    max_health: f32,
    #[init(val = 100.0)]
    health: f32,
    #[init(val = 4.6)]
    walk_speed: f32,
    #[init(val = 6.2)]
    sprint_speed: f32,
    #[init(val = 2.8)]
    crouch_speed: f32,
    #[init(val = 14.5)]
    gravity: f32,

    /// Nivel de dificultad: 0 = Recluta, 1 = Táctico, 2 = Veterano.
    #[init(val = 1)]
    difficulty_level: i32,

    ai_active: bool,
    is_dead: bool,
    is_crouching: bool,
    crouch_blend: f32,
    yaw: f32,
    aim_pitch: f32,

    state: AiTacticalState,
    state_timer: f64,
    reaction_timer: f64,
    burst_shots_left: i32,
    burst_pause_timer: f64,
    bursts_fired_exposed: i32,
    strafe_sign: f32,
    strafe_timer: f64,
    decision_seed: u32,

    seconds_without_los: f32,
    player_stationary_secs: f32,
    last_player_sample: Vector3,
    last_known_player_pos: Vector3,
    has_seen_player: bool,
    under_fire_alert: bool,
    current_cover_target: Vector3,
    has_cover_target: bool,

    cover_points: Vec<Vector3>,
    player_node: Option<Gd<Node3D>>,
    nav_agent: Option<Gd<NavigationAgent3D>>,
    collision_shape: Option<Gd<CollisionShape3D>>,
    visual_pivot: Option<Gd<Node3D>>,
    weapon_mount: Option<Gd<Node3D>>,
    weapon_rig: Option<Gd<WeaponRig>>,
}

#[godot_api]
impl ICharacterBody3D for EnemyAiCapsule {
    fn ready(&mut self) {
        self.health = self.max_health;
        self.state = AiTacticalState::PatrolAdvance;
        self.strafe_sign = 1.0;

        self.nav_agent = self.base().try_get_node_as::<NavigationAgent3D>("NavigationAgent3D");
        self.collision_shape = self.base().try_get_node_as::<CollisionShape3D>("CollisionShape3D");
        self.visual_pivot = self.base().try_get_node_as::<Node3D>("VisualPivot");
        self.weapon_mount = self.base().try_get_node_as::<Node3D>("WeaponMount");
        self.weapon_rig = self.base().try_get_node_as::<WeaponRig>("WeaponMount/WeaponRig");

        if let Some(mut rig) = self.weapon_rig.clone() {
            let body_obj: Gd<CollisionObject3D> = self.to_gd().upcast();
            rig.bind_mut().set_shooter_body(Some(body_obj));
            rig.bind_mut().set_base_damage(self.difficulty_damage());
        }
    }

    fn physics_process(&mut self, delta: f64) {
        let dt = delta as f32;
        if self.is_dead || !self.ai_active {
            return;
        }

        let target_crouch = if self.is_crouching { 1.0 } else { 0.0 };
        self.crouch_blend += (target_crouch - self.crouch_blend) * (dt * 10.0).min(1.0);
        self.apply_stance_heights();

        self.state_timer += delta;
        self.strafe_timer -= delta;
        if self.strafe_timer <= 0.0 {
            self.decision_seed = self.decision_seed.wrapping_add(1);
            self.strafe_sign = if self.decision_seed % 2 == 0 { 1.0 } else { -1.0 };
            self.strafe_timer = 1.1 + ((self.decision_seed % 4) as f64) * 0.35;
        }

        let Some(player) = self.player_node.clone() else {
            return;
        };
        let player_pos = player.get_global_position();
        let my_pos = self.base().get_global_position();

        // Detecta si el jugador está acampando sin moverse.
        if (player_pos - self.last_player_sample).length() < 0.35 {
            self.player_stationary_secs += dt;
        } else {
            self.player_stationary_secs = 0.0;
            self.last_player_sample = player_pos;
        }

        let ai_eye = my_pos + Vector3::new(0.0, if self.is_crouching { 0.92 } else { 1.42 }, 0.0);
        let player_chest = player_pos + Vector3::new(0.0, 1.05, 0.0);
        let player_rid = player
            .clone()
            .try_cast::<CollisionObject3D>()
            .ok()
            .map(|c| c.get_rid());
        let has_los = self.check_line_of_sight(ai_eye, player_chest, player_rid);

        if has_los {
            self.has_seen_player = true;
            self.last_known_player_pos = player_pos;
            self.seconds_without_los = 0.0;
        } else {
            self.seconds_without_los += dt;
        }

        self.update_tactical_fsm(delta, my_pos, player_pos, has_los);
        self.apply_movement_and_aim(dt, my_pos, player_pos, has_los);
    }
}

#[godot_api]
impl EnemyAiCapsule {
    #[signal]
    fn health_changed(current: f32, max_hp: f32);

    #[signal]
    fn died();

    #[signal]
    fn state_changed(state_name: GString);

    #[func]
    pub fn set_ai_active(&mut self, active: bool) {
        self.ai_active = active;
    }

    #[func]
    pub fn set_player_node(&mut self, player: Option<Gd<Node3D>>) {
        self.player_node = player;
    }

    #[func]
    pub fn set_difficulty(&mut self, level: i32) {
        self.difficulty_level = level.clamp(0, 2);
        if let Some(mut rig) = self.weapon_rig.clone() {
            rig.bind_mut().set_base_damage(self.difficulty_damage());
        }
    }

    #[func]
    pub fn clear_cover_points(&mut self) {
        self.cover_points.clear();
    }

    #[func]
    pub fn register_cover_point(&mut self, point: Vector3) {
        self.cover_points.push(point);
    }

    #[func]
    pub fn get_weapon_rig(&self) -> Option<Gd<WeaponRig>> {
        self.weapon_rig.clone()
    }

    #[func]
    pub fn get_health(&self) -> f32 {
        self.health
    }

    #[func]
    pub fn reset_for_round(&mut self, spawn_pos: Vector3, face_target: Vector3) {
        self.is_dead = false;
        self.health = self.max_health;
        self.is_crouching = false;
        self.crouch_blend = 0.0;
        self.has_seen_player = false;
        self.under_fire_alert = false;
        self.has_cover_target = false;
        self.seconds_without_los = 0.0;
        self.player_stationary_secs = 0.0;
        self.burst_shots_left = 0;
        self.burst_pause_timer = 0.0;
        self.bursts_fired_exposed = 0;
        self.last_known_player_pos = face_target;
        self.last_player_sample = face_target;

        self.base_mut().set_global_position(spawn_pos);
        self.base_mut().set_velocity(Vector3::ZERO);

        let to_target = Vector3::new(face_target.x - spawn_pos.x, 0.0, face_target.z - spawn_pos.z);
        if to_target.length_squared() > 0.001 {
            self.yaw = (-to_target.x).atan2(-to_target.z);
        } else {
            self.yaw = 0.0;
        }
        self.aim_pitch = 0.0;
        self.apply_stance_heights();
        self.apply_aim_transforms();

        if let Some(mut rig) = self.weapon_rig.clone() {
            rig.bind_mut().reset_ammo();
        }

        self.transition_to(AiTacticalState::PatrolAdvance);
        let hp = self.health;
        let max_hp = self.max_health;
        self.signals().health_changed().emit(hp, max_hp);
    }

    /// Reacciona al recibir impactos: gira hacia la amenaza y busca cobertura si está expuesto.
    #[func]
    pub fn apply_bullet_damage(&mut self, amount: f32, attacker_pos: Vector3) {
        if self.is_dead || amount <= 0.0 {
            return;
        }
        self.health = (self.health - amount).max(0.0);
        self.last_known_player_pos = attacker_pos;
        self.has_seen_player = true;
        self.under_fire_alert = true;

        let hp = self.health;
        let max_hp = self.max_health;
        self.signals().health_changed().emit(hp, max_hp);

        if self.health <= 0.0 {
            self.is_dead = true;
            self.ai_active = false;
            self.signals().died().emit();
            return;
        }

        // Si recibe fuego estando al descubierto, busca barrera o cubre su posición agachándose.
        if self.state != AiTacticalState::SeekCover && self.state != AiTacticalState::ReloadInCover {
            if self.health < 72.0 || (self.decision_seed % 2 == 0) {
                self.choose_cover_and_move(attacker_pos);
            } else {
                self.is_crouching = true;
            }
        }
    }

    fn transition_to(&mut self, next: AiTacticalState) {
        if self.state != next {
            self.state = next;
            self.state_timer = 0.0;
            let label = GString::from(next.label());
            self.signals().state_changed().emit(&label);
        }
    }

    fn difficulty_damage(&self) -> f32 {
        match self.difficulty_level {
            0 => 14.0,
            2 => 23.0,
            _ => 18.5,
        }
    }

    fn difficulty_reaction_time(&self) -> f64 {
        match self.difficulty_level {
            0 => 0.42,
            2 => 0.17,
            _ => 0.27,
        }
    }

    fn difficulty_turn_speed(&self) -> f32 {
        match self.difficulty_level {
            0 => 4.2,
            2 => 8.4,
            _ => 6.1,
        }
    }

    fn difficulty_base_spread(&self) -> f32 {
        match self.difficulty_level {
            0 => 2.8,
            2 => 1.05,
            _ => 1.75,
        }
    }

    fn update_tactical_fsm(
        &mut self,
        delta: f64,
        my_pos: Vector3,
        player_pos: Vector3,
        has_los: bool,
    ) {
        let (ammo_in_mag, reserve_ammo, is_reloading) = if let Some(rig) = self.weapon_rig.clone() {
            let r = rig.bind();
            (r.get_ammo_in_mag(), r.get_reserve_ammo(), r.is_weapon_reloading())
        } else {
            (30, 120, false)
        };

        if (ammo_in_mag <= 3 && reserve_ammo > 0) && self.state != AiTacticalState::ReloadInCover {
            if self.state != AiTacticalState::SeekCover {
                self.choose_cover_and_move(player_pos);
                return;
            }
        }

        match self.state {
            AiTacticalState::PatrolAdvance => {
                self.is_crouching = false;
                if has_los {
                    self.reaction_timer = self.difficulty_reaction_time();
                    self.burst_shots_left = 0;
                    self.burst_pause_timer = 0.0;
                    self.bursts_fired_exposed = 0;
                    self.transition_to(AiTacticalState::EngageBurst);
                    return;
                }

                // Avanza hacia cobertura intermedia o última posición conocida del jugador.
                if !self.has_cover_target || self.state_timer > 3.2 {
                    self.pick_advance_waypoint(my_pos, player_pos);
                } else if (my_pos - self.current_cover_target).length() < 1.1 {
                    self.pick_advance_waypoint(my_pos, player_pos);
                }

                let hp_ratio = self.health / self.max_health.max(1.0);
                if should_flank(self.seconds_without_los, self.player_stationary_secs, hp_ratio) {
                    self.pick_flank_waypoint(my_pos, player_pos);
                }
            }

            AiTacticalState::EngageBurst => {
                if !has_los {
                    if self.state_timer > 1.2 {
                        let hp_ratio = self.health / self.max_health.max(1.0);
                        if should_flank(self.seconds_without_los, self.player_stationary_secs, hp_ratio) {
                            self.pick_flank_waypoint(my_pos, player_pos);
                        } else {
                            self.transition_to(AiTacticalState::PatrolAdvance);
                        }
                    }
                    return;
                }

                if self.reaction_timer > 0.0 {
                    self.reaction_timer -= delta;
                    return;
                }

                if is_reloading {
                    self.choose_cover_and_move(player_pos);
                    return;
                }

                if self.burst_pause_timer > 0.0 {
                    self.burst_pause_timer -= delta;
                    return;
                }

                let dist = (player_pos - my_pos).length();
                if self.burst_shots_left <= 0 {
                    if self.bursts_fired_exposed >= 2 || self.under_fire_alert {
                        self.under_fire_alert = false;
                        self.choose_cover_and_move(player_pos);
                        return;
                    }
                    self.decision_seed = self.decision_seed.wrapping_add(1);
                    self.burst_shots_left =
                        select_burst_length(dist, self.difficulty_level, self.decision_seed);
                    self.is_crouching = dist > 11.0 && (self.decision_seed % 2 == 0);
                }

                // Dispara una bala de la ráfaga actual hacia el pecho/cabeza del jugador.
                self.fire_at_player(player_pos);
            }

            AiTacticalState::SeekCover => {
                self.is_crouching = false;
                let arrived = if let Some(nav) = self.nav_agent.clone() {
                    nav.is_navigation_finished()
                } else {
                    true
                };
                let dist_to_cover = (my_pos - self.current_cover_target).length();
                if arrived || dist_to_cover < 0.95 || self.state_timer > 4.2 {
                    self.is_crouching = true;
                    self.under_fire_alert = false;
                    if ammo_in_mag < 20 && reserve_ammo > 0 {
                        if let Some(mut rig) = self.weapon_rig.clone() {
                            rig.bind_mut().try_reload();
                        }
                    }
                    self.transition_to(AiTacticalState::ReloadInCover);
                }
            }

            AiTacticalState::ReloadInCover => {
                self.is_crouching = true;
                // Permanece agachado hasta completar recarga y recuperar compostura.
                if !is_reloading && self.state_timer >= 0.95 {
                    self.is_crouching = false;
                    self.bursts_fired_exposed = 0;
                    if has_los {
                        self.reaction_timer = self.difficulty_reaction_time() * 0.65;
                        self.transition_to(AiTacticalState::EngageBurst);
                    } else {
                        self.pick_flank_waypoint(my_pos, player_pos);
                    }
                }
            }

            AiTacticalState::Flank => {
                self.is_crouching = false;
                if has_los && self.state_timer > 0.65 {
                    self.reaction_timer = self.difficulty_reaction_time() * 0.7;
                    self.burst_shots_left = 0;
                    self.bursts_fired_exposed = 0;
                    self.transition_to(AiTacticalState::EngageBurst);
                    return;
                }
                let arrived = if let Some(nav) = self.nav_agent.clone() {
                    nav.is_navigation_finished()
                } else {
                    true
                };
                if arrived || self.state_timer > 4.5 {
                    self.transition_to(AiTacticalState::PatrolAdvance);
                }
            }
        }
    }

    fn apply_movement_and_aim(
        &mut self,
        dt: f32,
        my_pos: Vector3,
        player_pos: Vector3,
        has_los: bool,
    ) {
        let mut vel = self.base().get_velocity();
        if !self.base().is_on_floor() {
            vel.y -= self.gravity * dt;
        } else if vel.y < 0.0 {
            vel.y = -0.1;
        }

        let mut desired_vel = Vector3::ZERO;
        match self.state {
            AiTacticalState::PatrolAdvance | AiTacticalState::SeekCover | AiTacticalState::Flank => {
                if let Some(mut nav) = self.nav_agent.clone() {
                    if !nav.is_navigation_finished() {
                        let next_pt = nav.get_next_path_position();
                        let dir = Vector3::new(next_pt.x - my_pos.x, 0.0, next_pt.z - my_pos.z);
                        if dir.length_squared() > 0.002 {
                            let spd = if self.state == AiTacticalState::SeekCover {
                                self.sprint_speed
                            } else {
                                self.walk_speed
                            };
                            desired_vel = dir.normalized() * spd;
                        }
                    }
                }
            }
            AiTacticalState::EngageBurst => {
                // Desplazamiento lateral táctico para no ser un blanco estático.
                let xform = self.base().get_global_transform();
                let right = xform.basis.col_a();
                let flat_right = Vector3::new(right.x, 0.0, right.z).normalized_or_zero();
                let spd = if self.is_crouching {
                    self.crouch_speed * 0.55
                } else {
                    self.walk_speed * 0.62
                };
                desired_vel = flat_right * (self.strafe_sign * spd);
            }
            AiTacticalState::ReloadInCover => {
                desired_vel = Vector3::ZERO;
            }
        }

        vel.x += (desired_vel.x - vel.x) * (dt * 9.5).min(1.0);
        vel.z += (desired_vel.z - vel.z) * (dt * 9.5).min(1.0);
        self.base_mut().set_velocity(vel);
        self.base_mut().move_and_slide();

        // Orientación suave hacia el jugador o hacia la dirección de carrera al huir a cobertura.
        let look_target = if has_los || self.state == AiTacticalState::EngageBurst {
            player_pos
        } else if desired_vel.length_squared() > 0.25 {
            my_pos + desired_vel
        } else {
            self.last_known_player_pos
        };

        let dx = look_target.x - my_pos.x;
        let dz = look_target.z - my_pos.z;
        if dx * dx + dz * dz > 0.01 {
            let target_yaw = (-dx).atan2(-dz);
            self.yaw = compute_smooth_yaw_step(self.yaw, target_yaw, self.difficulty_turn_speed(), dt);
        }

        let dy = (look_target.y + 1.05) - (my_pos.y + 1.24);
        let flat_d = (dx * dx + dz * dz).sqrt().max(0.5);
        let target_pitch = dy.atan2(flat_d).clamp(-0.65, 0.65);
        self.aim_pitch += (target_pitch - self.aim_pitch) * (dt * 8.0).min(1.0);
        self.apply_aim_transforms();
    }

    fn fire_at_player(&mut self, player_pos: Vector3) {
        let Some(mut rig) = self.weapon_rig.clone() else {
            return;
        };

        let my_pos = self.base().get_global_position();
        let aim_origin = my_pos + Vector3::new(0.0, if self.is_crouching { 0.85 } else { 1.25 }, 0.0);
        // Sutil oscilación humana en la puntería para no acertar el 100% de las balas en el centro exacto.
        let sway = ((self.state_timer * 3.1).sin() as f32) * 0.22;
        let aim_target = player_pos + Vector3::new(sway, 1.02, 0.0);
        let aim_dir = (aim_target - aim_origin).normalized_or_zero();

        let vel = self.base().get_velocity();
        let speed_ratio = Vector2::new(vel.x, vel.z).length() / self.walk_speed.max(0.1);
        let heat = rig.bind().get_recoil_heat();
        let spread = calculate_effective_spread_deg(
            self.difficulty_base_spread(),
            speed_ratio,
            self.is_crouching,
            heat,
        );

        let fired = rig.bind_mut().try_fire(aim_origin, aim_dir, spread);
        if fired {
            self.burst_shots_left -= 1;
            if self.burst_shots_left <= 0 {
                self.bursts_fired_exposed += 1;
                self.burst_pause_timer = match self.difficulty_level {
                    0 => 0.58,
                    2 => 0.28,
                    _ => 0.42,
                };
            }
        }
    }

    fn choose_cover_and_move(&mut self, threat_pos: Vector3) {
        let my_pos = self.base().get_global_position();
        let player_rid = self
            .player_node
            .as_ref()
            .and_then(|p| p.clone().try_cast::<CollisionObject3D>().ok())
            .map(|c| c.get_rid());

        let mut best_pos = my_pos;
        let mut best_score = f32::NEG_INFINITY;

        let points = self.cover_points.clone();
        for (idx, pt) in points.iter().enumerate() {
            // Comprueba mediante raycast real si la barrera bloquea la línea de visión estando agachado.
            let cover_eye = *pt + Vector3::new(0.0, 0.85, 0.0);
            let threat_eye = threat_pos + Vector3::new(0.0, 1.35, 0.0);
            let has_los_from_cover = self.check_line_of_sight(cover_eye, threat_eye, player_rid);
            let blocks_los = !has_los_from_cover;

            let mut score = score_cover_candidate(
                [pt.x, pt.y, pt.z],
                [my_pos.x, my_pos.y, my_pos.z],
                [threat_pos.x, threat_pos.y, threat_pos.z],
                blocks_los,
                11.5,
            );
            // Pequeña variación para que la IA no repita exactamente la misma esquina cada ronda.
            if (idx as u32) == (self.decision_seed % (points.len().max(1) as u32)) {
                score += 4.5;
            }

            if score > best_score {
                best_score = score;
                best_pos = *pt;
            }
        }

        if points.is_empty() {
            // Respaldo si aún no se registraron coberturas: retrocede lateralmente.
            let away = (my_pos - threat_pos).normalized_or_zero();
            best_pos = my_pos + away * 5.0;
        }

        self.set_nav_target(best_pos);
        self.transition_to(AiTacticalState::SeekCover);
    }

    fn pick_advance_waypoint(&mut self, my_pos: Vector3, player_pos: Vector3) {
        if self.cover_points.is_empty() {
            self.set_nav_target(player_pos);
            return;
        }
        self.decision_seed = self.decision_seed.wrapping_add(1);
        let mut best = player_pos;
        let mut best_score = f32::NEG_INFINITY;

        for (i, pt) in self.cover_points.iter().enumerate() {
            let d_threat = (*pt - player_pos).length();
            let d_me = (*pt - my_pos).length();
            let mut s = -(d_threat - 10.0).abs() * 1.4 - d_me * 0.7;
            if d_me < 2.2 {
                s -= 25.0;
            }
            if ((i as u32) + self.decision_seed) % 3 == 0 {
                s += 3.5;
            }
            if s > best_score {
                best_score = s;
                best = *pt;
            }
        }
        self.set_nav_target(best);
    }

    fn pick_flank_waypoint(&mut self, my_pos: Vector3, player_pos: Vector3) {
        self.decision_seed = self.decision_seed.wrapping_add(1);
        let to_player = (player_pos - my_pos).normalized_or_zero();
        let lateral = Vector3::new(-to_player.z, 0.0, to_player.x)
            * (if self.decision_seed % 2 == 0 { 1.0 } else { -1.0 });
        let desired_flank = (my_pos + player_pos) * 0.5 + lateral * 8.5;

        let mut best = desired_flank;
        if !self.cover_points.is_empty() {
            let mut min_d = f32::INFINITY;
            for pt in &self.cover_points {
                let d = (*pt - desired_flank).length();
                if d < min_d && (*pt - my_pos).length() > 3.0 {
                    min_d = d;
                    best = *pt;
                }
            }
        }

        self.set_nav_target(best);
        self.transition_to(AiTacticalState::Flank);
    }

    fn set_nav_target(&mut self, target: Vector3) {
        self.current_cover_target = target;
        self.has_cover_target = true;
        if let Some(mut nav) = self.nav_agent.clone() {
            nav.set_target_position(target);
        }
    }

    fn check_line_of_sight(
        &self,
        from_pos: Vector3,
        to_pos: Vector3,
        player_rid: Option<Rid>,
    ) -> bool {
        let Some(world) = self.base().get_world_3d() else {
            return false;
        };
        let Some(mut space) = world.get_direct_space_state() else {
            return false;
        };

        let mut exclude = Array::<Rid>::new();
        exclude.push(self.base().get_rid());

        let mut query = PhysicsRayQueryParameters3D::new_gd();
        query.set_from(from_pos);
        query.set_to(to_pos);
        query.set_exclude(&exclude);
        query.set_collide_with_bodies(true);
        query.set_collide_with_areas(false);

        let result = space.intersect_ray(&query);
        if result.is_empty() {
            return true;
        }
        if let (Some(target_rid), Some(rid_var)) = (player_rid, result.get("rid")) {
            if let Ok(hit_rid) = rid_var.try_to::<Rid>() {
                return hit_rid == target_rid;
            }
        }
        false
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
        if let Some(mut w_mount) = self.weapon_mount.clone() {
            let w_y = 1.24 - 0.42 * blend;
            w_mount.set_position(Vector3::new(0.31, w_y, -0.27));
        }
    }

    fn apply_aim_transforms(&mut self) {
        let yaw = self.yaw;
        self.base_mut().set_rotation(Vector3::new(0.0, yaw, 0.0));
        if let Some(mut w_mount) = self.weapon_mount.clone() {
            w_mount.set_rotation(Vector3::new(self.aim_pitch, 0.0, 0.0));
        }
    }
}
