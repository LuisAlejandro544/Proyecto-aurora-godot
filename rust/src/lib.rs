//! Punto de entrada principal de GDExtension (`shooter_core`) en Rust para Godot 4.7.
//!
//! Registra los módulos desacoplados del juego:
//! - `combat_math`: cálculos puros de daño, dispersión y puntuación de cobertura.
//! - `weapon_rig`: rifle clásico 7.62, fogonazo, trazadoras y casquillos expulsados.
//! - `player_capsule`: controlador móvil/escritorio de la cápsula del jugador.
//! - `enemy_ai_capsule`: IA local táctica con navegación, coberturas y flanqueo.

use godot::prelude::*;

mod combat_math;
mod enemy_ai_capsule;
mod player_capsule;
mod weapon_rig;

/// Estructura marcadora que registra la librería dinámica en Godot 4.7.
struct ShooterCoreExtension;

/// Macro de `gdext` que expone el símbolo `gdext_rust_init` configurado en `godot/shooter_core.gdextension`.
#[gdextension]
unsafe impl ExtensionLibrary for ShooterCoreExtension {}
