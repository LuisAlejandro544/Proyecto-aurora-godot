//! Punto de entrada mínimo de GDExtension para conectar Rust con Godot 4.7.
//!
//! Este archivo NO contiene todavía lógica ni mecánicas del juego; únicamente
//! registra la librería dinámica (`libshooter_core.so`) para que Godot pueda
//! inicializarla correctamente al compilar el APK Debug en Android.

use godot::prelude::*;

/// Estructura marcadora que representa la extensión principal en Rust.
struct ShooterCoreExtension;

/// Macro de `gdext` que expone el símbolo `gdext_rust_init` requerido por
/// el archivo `godot/shooter_core.gdextension`.
#[gdextension]
unsafe impl ExtensionLibrary for ShooterCoreExtension {}
