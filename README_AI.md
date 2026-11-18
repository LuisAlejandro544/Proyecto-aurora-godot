# README_AI.md — Guía Rápida de Onboarding para Asistentes

Resumen operativo para entender en menos de 60 segundos cómo interactúan Godot 4.7, Rust (`gdext 0.5.5`) y Blender 5.2.4 en **Iron Vector**.

---

## Mapa de Contratos Entre Módulos

### 1. Clases Rust Expuestas a Godot (`rust/src/`)
| Clase (`GodotClass`) | Base | Archivo | Señales Emitidas | Métodos Clave (`#[func]`) |
| :--- | :--- | :--- | :--- | :--- |
| `WeaponRig` | `Node3D` | `weapon_rig.rs` | `ammo_changed`, `reload_started`, `reload_finished`, `weapon_fired`, `hit_target` | `configure_parts`, `set_fx_scenes`, `set_fx_container`, `set_shooter_body`, `try_fire`, `try_reload`, `reset_ammo` |
| `TracerProjectile` | `Node3D` | `weapon_rig.rs` | — | `launch` |
| `EjectedCasing` | `Node3D` | `weapon_rig.rs` | — | `play_eject_animation` |
| `PlayerCapsule` | `CharacterBody3D` | `player_capsule.rs` | `health_changed`, `died`, `hit_confirmed`, `took_damage` | `set_move_input`, `add_look_delta`, `set_firing`, `set_crouching`, `set_sprinting`, `trigger_jump`, `trigger_reload`, `set_camera_mode_first_person`, `reset_for_round`, `apply_bullet_damage` |
| `EnemyAiCapsule` | `CharacterBody3D` | `enemy_ai_capsule.rs` | `health_changed`, `died`, `state_changed` | `set_ai_active`, `set_player_node`, `set_difficulty`, `clear_cover_points`, `register_cover_point`, `reset_for_round`, `apply_bullet_damage` |

### 2. Nodos Esperados Dentro de los `.glb` Generados por Blender
- `classic_assault_rifle.glb`: `AnimationPlayer` (`idle`, `fire`, `reload`), `MuzzlePoint`, `EjectPoint`, `SightPoint`, `BoltCarrier`, `Magazine`.
- `muzzle_flash_fx.glb`: `AnimationPlayer` (`flash_burst`).
- `shell_casing_eject.glb`: `AnimationPlayer` (`casing_eject`).
- `bullet_projectile.glb`: malla estática `BulletJacket` + `BulletTracerBase`.
- `ruined_city_arena.glb`: colisionadores `-col`, `SpawnPlayer`, `SpawnAI`, más estructuras nombradas `Barrier_*`, `Sandbags_*`, `Wreck_*`, `CrateStack_*`, `Building_*`.

### 3. Secuencia de Arranque en Runtime (`godot/scripts/game_manager.gd`)
1. `GameManager._ready()` oculta personajes/HUD y lanza la corrutina `_bootstrap_assets_and_world()`.
2. `AssetLoader.load_all_assets()` carga en paralelo/hilo los 5 `.glb` desde `res://assets/models/`.
3. `ArenaController.build_arena()` instancia la ciudad, lee `SpawnPlayer`/`SpawnAI`, extrae puntos de cobertura y hornea `NavigationMesh` en segundo plano (`bake_navigation_mesh(true)`).
4. `CapsuleVisualBuilder.build_capsule_and_weapon()` construye la cápsula estándar, visor y manos de cada personaje, monta el rifle rotado `+90°` en Y, acopla el fogonazo a `MuzzlePoint` y conecta todo al `WeaponRig` en Rust.
5. `GameManager` registra los puntos de cobertura en `EnemyAiCapsule` y muestra `UI_Menu` con la cámara orbital sobre el escenario vivo.

### 4. Comandos de Verificación
- **Web Hello World**: `npx --yes tsx --test src/greeting.test.ts` y `npm run build` / `npm run lint`.
- **Tests puros de Rust (en CI o entorno con Cargo)**: `cd rust && cargo test --lib`.
- **Assets Blender headless (si Blender está disponible)**: `blender --background --python scripts/blender/<script>.py`.
