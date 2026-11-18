//! Punto de entrada principal de GDExtension (`shooter_core`) en Rust para Godot 4.7.
//!
//! Registra los módulos desacoplados del juego:
//! - `combat_math`: cálculos puros de daño, dispersión ADS, curva de slide y granadas.
//! - `grenade`: física de lanzamiento/rebote de granadas de fragmentación y explosión 3D.
//! - `weapon_rig`: rifle clásico 7.62, apuntado/recarga ADS, fogonazo, trazadoras y casquillos.
//! - `player_capsule`: controlador táctico con slide, slide-cancel, lanzamiento (dive), ADS y granada.
//! - `enemy_ai_capsule`: IA local táctica con navegación, coberturas, flanqueo y uso de granadas.

use godot::prelude::*;

mod combat_math;
mod enemy_ai_capsule;
mod grenade;
mod player_capsule;
mod weapon_rig;

/// Estructura marcadora que registra la librería dinámica en Godot 4.7.
struct ShooterCoreExtension;

/// Macro de `gdext` que expone el símbolo `gdext_rust_init` configurado en `godot/shooter_core.gdextension`.
#[gdextension]
unsafe impl ExtensionLibrary for ShooterCoreExtension {}
