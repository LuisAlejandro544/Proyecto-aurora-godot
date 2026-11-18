//! Lógica matemática pura de combate, dispersión balística y evaluación táctica de coberturas.
//!
//! Este módulo NO depende de nodos de Godot para mantenerse 100% modular, determinista
//! y fácil de verificar con pruebas unitarias (`cargo test`) sin necesidad de motor gráfico.

use std::f32::consts::{PI, TAU};

/// Calcula el daño efectivo de un disparo 7.62 según la distancia al objetivo.
/// Mantiene letalidad constante hasta los 22 metros y aplica caída progresiva hasta un mínimo del 55%.
pub fn calculate_damage_with_falloff(base_damage: f32, distance_m: f32) -> f32 {
    if base_damage <= 0.0 {
        return 0.0;
    }
    let clamped_dist = distance_m.max(0.0);
    if clamped_dist <= 22.0 {
        return base_damage;
    }
    let falloff_span = (clamped_dist - 22.0) / 38.0;
    let multiplier = (1.0 - falloff_span * 0.45).clamp(0.55, 1.0);
    base_damage * multiplier
}

/// Calcula la dispersión angular total en grados considerando movimiento, agachado y calor de retroceso.
/// Agacharse detrás de coberturas mejora notablemente la precisión del rifle.
pub fn calculate_effective_spread_deg(
    base_spread_deg: f32,
    move_speed_ratio: f32,
    is_crouching: bool,
    recoil_heat: f32,
) -> f32 {
    let move_penalty = move_speed_ratio.clamp(0.0, 1.5) * 1.45;
    let stance_factor = if is_crouching { 0.62 } else { 1.0 };
    let recoil_penalty = recoil_heat.clamp(0.0, 1.0) * 2.1;
    ((base_spread_deg + move_penalty) * stance_factor + recoil_penalty).clamp(0.25, 8.5)
}

/// Genera un desvío angular determinista `(yaw_rad, pitch_rad)` dentro de un cono circular
/// sin depender de librerías externas de números aleatorios.
pub fn compute_spread_angles_rad(shot_index: u32, spread_deg: f32) -> (f32, f32) {
    if spread_deg <= 0.001 {
        return (0.0, 0.0);
    }
    // Hash multiplicativo de Knuth + ángulo áureo para distribución uniforme en el cono.
    let hashed = shot_index.wrapping_mul(2_654_435_761).wrapping_add(1_013_904_223);
    let radial_unit = ((hashed & 0xFFFF) as f32) / 65535.0;
    let angle = ((hashed >> 16) as f32) * 2.399_963_2; // ~ángulo áureo en radianes
    let max_rad = spread_deg.to_radians();
    let radius = max_rad * radial_unit.sqrt();
    (radius * angle.cos(), radius * angle.sin())
}

/// Determina cuántos disparos hará la IA en una ráfaga realista según distancia y dificultad.
/// Evita que la IA dispare en automático infinito como un robot.
pub fn select_burst_length(distance_m: f32, difficulty_level: i32, shot_seed: u32) -> i32 {
    let extra = (shot_seed % 3) as i32;
    let base = match difficulty_level {
        0 => {
            if distance_m < 14.0 { 3 } else { 2 }
        }
        2 => {
            if distance_m < 14.0 { 5 } else { 3 }
        }
        _ => {
            if distance_m < 14.0 { 4 } else { 2 }
        }
    };
    (base + extra - 1).clamp(2, 6)
}

/// Evalúa la calidad táctica de un punto candidato de cobertura (`[x, y, z]`).
/// Prioriza puntos que realmente bloquean la línea de visión del jugador (`blocks_los == true`),
/// se encuentran a una distancia de combate saludable (~10-18m) y no están demasiado lejos de la IA.
pub fn score_cover_candidate(
    candidate_pos: [f32; 3],
    ai_pos: [f32; 3],
    threat_pos: [f32; 3],
    blocks_los: bool,
    preferred_combat_dist: f32,
) -> f32 {
    let dx_ai = candidate_pos[0] - ai_pos[0];
    let dz_ai = candidate_pos[2] - ai_pos[2];
    let dist_from_ai = (dx_ai * dx_ai + dz_ai * dz_ai).sqrt();

    let dx_th = candidate_pos[0] - threat_pos[0];
    let dz_th = candidate_pos[2] - threat_pos[2];
    let dist_from_threat = (dx_th * dx_th + dz_th * dz_th).sqrt();

    // Gran bonificación si la barrera u obstáculo corta de verdad la línea de tiro del jugador.
    let mut score = if blocks_los { 110.0 } else { -35.0 };

    // Penaliza puntos extremadamente pegados al enemigo (peligro de asalto directo) o lejanos.
    let dist_error = (dist_from_threat - preferred_combat_dist).abs();
    score -= dist_error * 2.1;

    if dist_from_threat < 4.5 {
        score -= 45.0;
    }

    // Penaliza levemente desplazamientos largos para preferir cobertura cercana alcanzable a tiempo.
    score -= dist_from_ai * 1.35;

    score
}

/// Decide si la IA debe iniciar una maniobra de flanqueo porque el jugador permanece escondido
/// o atrincherado sin línea de visión directa durante demasiado tiempo.
pub fn should_flank(seconds_without_los: f32, threat_stationary_secs: f32, hp_ratio: f32) -> bool {
    if hp_ratio < 0.25 {
        // Con muy poca vida prefiere defender o recargar en cobertura antes que lanzarse a campo abierto.
        return false;
    }
    seconds_without_los >= 4.2 || (seconds_without_los >= 2.6 && threat_stationary_secs >= 3.0)
}

/// Calcula la diferencia mínima con signo entre dos ángulos en radianes dentro de `[-PI, PI]`.
pub fn angle_difference(from_rad: f32, to_rad: f32) -> f32 {
    let mut diff = (to_rad - from_rad) % TAU;
    if diff > PI {
        diff -= TAU;
    } else if diff < -PI {
        diff += TAU;
    }
    diff
}

/// Avanza el ángulo horizontal (`yaw`) hacia un objetivo con velocidad angular humana limitada.
pub fn compute_smooth_yaw_step(
    current_yaw: f32,
    target_yaw: f32,
    max_turn_speed_rad: f32,
    dt: f32,
) -> f32 {
    let diff = angle_difference(current_yaw, target_yaw);
    let max_step = (max_turn_speed_rad.max(0.0)) * (dt.max(0.0));
    current_yaw + diff.clamp(-max_step, max_step)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_stays_full_close_and_decays_far() {
        assert!((calculate_damage_with_falloff(24.0, 10.0) - 24.0).abs() < 1e-4);
        let far = calculate_damage_with_falloff(24.0, 60.0);
        assert!(far < 24.0 && far >= 24.0 * 0.54);
    }

    #[test]
    fn crouching_reduces_weapon_spread() {
        let standing = calculate_effective_spread_deg(1.2, 0.5, false, 0.3);
        let crouching = calculate_effective_spread_deg(1.2, 0.5, true, 0.3);
        assert!(crouching < standing);
    }

    #[test]
    fn cover_scoring_prefers_los_blocking_positions() {
        let ai_pos = [0.0, 0.0, -10.0];
        let threat = [0.0, 0.0, 10.0];
        let protected = score_cover_candidate([2.0, 0.0, -4.0], ai_pos, threat, true, 12.0);
        let exposed = score_cover_candidate([0.0, 0.0, -8.0], ai_pos, threat, false, 12.0);
        assert!(protected > exposed + 50.0);
    }

    #[test]
    fn flank_triggers_when_player_camps_hidden_and_ai_is_healthy() {
        assert!(should_flank(4.5, 1.0, 0.8));
        assert!(!should_flank(4.5, 1.0, 0.2));
        assert!(!should_flank(1.2, 0.5, 0.9));
    }

    #[test]
    fn smooth_yaw_takes_shortest_arc_across_pi_boundary() {
        let current = PI - 0.1;
        let target = -PI + 0.1;
        let next = compute_smooth_yaw_step(current, target, 4.0, 0.05);
        // Debe cruzar +PI hacia adelante en lugar de dar toda la vuelta por 0.
        assert!(next > current);
    }
}
