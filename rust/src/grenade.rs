//! Lógica física y visual de granadas de fragmentación (`FragGrenade`) y su explosión 3D (`ExplosionFx`).
//!
//! Implementa lanzamiento parabólico con rebote real mediante raycasts 3D contra muros y suelo,
//! giro en vuelo mediante animación `grenade_spin`, temporizador de espoleta, explosión 3D
//! (`explode_burst`) con comprobación de oclusión por cobertura (las barreras protegen de la
//! metralla) y daño radial con caída progresiva sobre cualquier combatiente en rango.

use godot::classes::{
    AnimationPlayer, CollisionObject3D, INode3D, Node3D, PackedScene, PhysicsRayQueryParameters3D,
};
use godot::prelude::*;

use crate::combat_math::calculate_grenade_damage;

/// Entidad efímera que reproduce la animación `explode_burst` de `grenade_explosion_fx.glb`
/// y se libera automáticamente al terminar la onda expansiva.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct ExplosionFx {
    base: Base<Node3D>,
    #[init(val = 0.72)]
    ttl: f64,
}

#[godot_api]
impl INode3D for ExplosionFx {
    fn process(&mut self, delta: f64) {
        self.ttl -= delta;
        if self.ttl <= 0.0 {
            self.base_mut().queue_free();
        }
    }
}

#[godot_api]
impl ExplosionFx {
    pub fn start_burst(&mut self, anim_player: Option<Gd<AnimationPlayer>>) {
        if let Some(mut anim) = anim_player {
            play_matching_anim(&mut anim, "explode_burst");
        }
        self.ttl = 0.72;
    }
}

/// Granada de fragmentación lanzada por el jugador o la IA enemiga.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct FragGrenade {
    base: Base<Node3D>,
    velocity: Vector3,
    #[init(val = 16.8)]
    gravity: f32,
    #[init(val = 1.65)]
    fuse_timer: f64,
    #[init(val = 0.42)]
    bounce_restitution: f32,
    #[init(val = 6.5)]
    blast_radius: f32,
    #[init(val = 95.0)]
    max_damage: f32,
    bounces: i32,
    has_detonated: bool,

    thrower_rid: Option<Rid>,
    explosion_scene: Option<Gd<PackedScene>>,
    fx_container: Option<Gd<Node3D>>,
    spin_anim: Option<Gd<AnimationPlayer>>,
}

#[godot_api]
impl INode3D for FragGrenade {
    fn physics_process(&mut self, delta: f64) {
        if self.has_detonated {
            return;
        }
        let dt = delta as f32;
        self.fuse_timer -= delta;

        if let Some(mut anim) = self.spin_anim.clone() {
            if self.velocity.length_squared() > 0.35 && !anim.is_playing() {
                play_matching_anim(&mut anim, "grenade_spin");
            }
        }

        self.velocity.y -= self.gravity * dt;
        let start_pos = self.base().get_global_position();
        let next_pos = start_pos + self.velocity * dt;

        // Barrido físico por raycast para rebotar limpiamente contra suelo, barreras y muros.
        let mut final_pos = next_pos;
        if let Some(hit) = self.sweep_segment(start_pos, next_pos) {
            let mut hit_pos = next_pos;
            let mut hit_normal = Vector3::UP;
            if let Some(p_var) = hit.get("position") {
                if let Ok(p) = p_var.try_to::<Vector3>() {
                    hit_pos = p;
                }
            }
            if let Some(n_var) = hit.get("normal") {
                if let Ok(n) = n_var.try_to::<Vector3>() {
                    if n.length_squared() > 0.001 {
                        hit_normal = n.normalized();
                    }
                }
            }

            final_pos = hit_pos + hit_normal * 0.075;
            let vn = self.velocity.dot(hit_normal);
            if vn < 0.0 {
                let v_normal = hit_normal * vn;
                let v_tangent = self.velocity - v_normal;
                self.velocity = v_tangent * 0.78 - v_normal * self.bounce_restitution;
                if self.velocity.length() < 0.45 && hit_normal.y > 0.6 {
                    self.velocity = Vector3::ZERO;
                }
            }
            self.bounces += 1;
        }

        self.base_mut().set_global_position(final_pos);

        if self.fuse_timer <= 0.0 {
            self.detonate();
        }
    }
}

#[godot_api]
impl FragGrenade {
    /// Emitida justo en el instante de detonación para reproducir audio 3D en el epicentro.
    #[signal]
    fn detonated(pos: Vector3);

    /// Inicializa trayectoria, animación y escena de explosión de la granada.
    pub fn launch(
        &mut self,
        start_pos: Vector3,
        initial_velocity: Vector3,
        thrower_rid: Option<Rid>,
        explosion_scene: Option<Gd<PackedScene>>,
        fx_container: Option<Gd<Node3D>>,
        spin_anim: Option<Gd<AnimationPlayer>>,
    ) {
        self.base_mut().set_global_position(start_pos);
        self.velocity = initial_velocity;
        self.thrower_rid = thrower_rid;
        self.explosion_scene = explosion_scene;
        self.fx_container = fx_container;
        self.spin_anim = spin_anim;
        self.fuse_timer = 1.65;
        self.bounces = 0;
        self.has_detonated = false;

        if let Some(mut anim) = self.spin_anim.clone() {
            play_matching_anim(&mut anim, "grenade_spin");
        }
    }

    fn sweep_segment(&self, from: Vector3, to: Vector3) -> Option<VarDictionary> {
        if (to - from).length_squared() < 0.000001 {
            return None;
        }
        let world = self.base().get_world_3d()?;
        let mut space = world.get_direct_space_state()?;

        let mut exclude = Array::<Rid>::new();
        // Ignora el cuerpo del lanzador al salir del pecho/mano para evitar auto-rebote inmediato.
        if self.bounces == 0 && self.fuse_timer > 1.38 {
            if let Some(rid) = self.thrower_rid {
                exclude.push(rid);
            }
        }

        let mut query = PhysicsRayQueryParameters3D::new_gd();
        query.set_from(from);
        query.set_to(to);
        query.set_exclude(&exclude);
        query.set_collide_with_bodies(true);
        query.set_collide_with_areas(false);

        let hit = space.intersect_ray(&query);
        if hit.is_empty() {
            None
        } else {
            Some(hit)
        }
    }

    fn detonate(&mut self) {
        if self.has_detonated {
            return;
        }
        self.has_detonated = true;
        let blast_pos = self.base().get_global_position();

        self.spawn_explosion_fx(blast_pos);
        self.apply_radial_blast_damage(blast_pos);

        self.signals().detonated().emit(blast_pos);
        self.base_mut().queue_free();
    }

    fn spawn_explosion_fx(&self, blast_pos: Vector3) {
        let Some(exp_scene) = self.explosion_scene.clone() else {
            return;
        };
        let parent_container = self.fx_container.clone().or_else(|| {
            self.base()
                .get_tree_or_null()
                .and_then(|t| t.get_current_scene())
                .and_then(|s| s.try_cast::<Node3D>().ok())
        });
        let Some(mut container) = parent_container else {
            return;
        };
        let Some(instance) = exp_scene.instantiate() else {
            return;
        };
        let Ok(mut exp_model) = instance.try_cast::<Node3D>() else {
            return;
        };

        let mut wrapper = ExplosionFx::new_alloc();
        wrapper.set_name("GrenadeExplosionFX");
        wrapper.add_child(&exp_model);
        container.add_child(&wrapper);
        wrapper.set_global_position(blast_pos);

        let anim = find_animation_player_recursive(&exp_model);
        wrapper.bind_mut().start_burst(anim);
    }

    fn apply_radial_blast_damage(&self, blast_pos: Vector3) {
        let Some(tree) = self.base().get_tree_or_null() else {
            return;
        };
        let blast_origin = blast_pos + Vector3::new(0.0, 0.25, 0.0);
        let combatants = tree.get_nodes_in_group("combatants");

        for node in combatants.iter_shared() {
            let Ok(target_3d) = node.clone().try_cast::<Node3D>() else {
                continue;
            };
            let target_pos = target_3d.get_global_position();
            let target_chest = target_pos + Vector3::new(0.0, 0.85, 0.0);
            let dist = (target_chest - blast_origin).length();
            if dist > self.blast_radius {
                continue;
            }

            let target_rid = target_3d
                .clone()
                .try_cast::<CollisionObject3D>()
                .ok()
                .map(|c| c.get_rid());

            // Comprueba línea de visión de metralla (pecho o cabeza); las barreras Jersey protegen.
            let exposed = dist < 0.85
                || self.has_blast_los(blast_origin, target_chest, target_rid)
                || self.has_blast_los(
                    blast_origin,
                    target_pos + Vector3::new(0.0, 1.38, 0.0),
                    target_rid,
                );
            if !exposed {
                continue;
            }

            let dmg = calculate_grenade_damage(self.max_damage, dist, self.blast_radius);
            if dmg >= 1.0 && node.has_method("apply_bullet_damage") {
                let mut target_obj: Gd<Object> = node.upcast();
                target_obj.call(
                    "apply_bullet_damage",
                    &[dmg.to_variant(), blast_pos.to_variant()],
                );
            }
        }
    }

    fn has_blast_los(&self, from: Vector3, to: Vector3, target_rid: Option<Rid>) -> bool {
        let Some(world) = self.base().get_world_3d() else {
            return false;
        };
        let Some(mut space) = world.get_direct_space_state() else {
            return false;
        };

        let mut query = PhysicsRayQueryParameters3D::new_gd();
        query.set_from(from);
        query.set_to(to);
        query.set_collide_with_bodies(true);
        query.set_collide_with_areas(false);

        let hit = space.intersect_ray(&query);
        if hit.is_empty() {
            return true;
        }
        if let (Some(wanted_rid), Some(rid_var)) = (target_rid, hit.get("rid")) {
            if let Ok(hit_rid) = rid_var.try_to::<Rid>() {
                return hit_rid == wanted_rid;
            }
        }
        false
    }
}

/// Busca recursivamente el `AnimationPlayer` generado al importar un modelo `.glb`.
pub fn find_animation_player_recursive(node: &Gd<Node3D>) -> Option<Gd<AnimationPlayer>> {
    if let Ok(anim) = node.clone().try_cast::<AnimationPlayer>() {
        return Some(anim);
    }
    if let Some(direct) = node.try_get_node_as::<AnimationPlayer>("AnimationPlayer") {
        return Some(direct);
    }
    let count = node.get_child_count();
    for i in 0..count {
        if let Some(child) = node.get_child(i) {
            if let Ok(anim) = child.clone().try_cast::<AnimationPlayer>() {
                return Some(anim);
            }
            if let Ok(child_3d) = child.try_cast::<Node3D>() {
                if let Some(found) = find_animation_player_recursive(&child_3d) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Reproduce una animación por coincidencia exacta o sufijo de pista NLA.
fn play_matching_anim(anim: &mut Gd<AnimationPlayer>, target: &str) {
    if anim.has_animation(target) {
        anim.stop();
        anim.play_ex().name(target).done();
        return;
    }
    let list = anim.get_animation_list();
    for idx in 0..list.len() {
        let candidate = list.get(idx).unwrap_or_default().to_string();
        if candidate.ends_with(target) {
            anim.stop();
            anim.play_ex().name(&candidate).done();
            return;
        }
    }
}
