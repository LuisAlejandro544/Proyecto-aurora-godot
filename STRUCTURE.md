# Estructura del Proyecto (`STRUCTURE.md`)

Organización modular de carpetas y propósito de cada archivo en el repositorio.

```text
.
├── .github/workflows/
│   ├── blender-assets.yml        # Action manual: ejecuta Blender 5.2.4 y publica Release con Tag
│   └── android-debug-apk.yml     # Action manual: baja .glb del Release, compila Rust y exporta APK Debug
├── scripts/blender/
│   ├── scene_utils.py            # Utilidades compartidas (materiales PBR, primitivas, NLA y GLB)
│   ├── weapon_classic_rifle.py   # Genera classic_assault_rifle.glb (idle, fire, reload)
│   ├── fx_muzzle_flash.py        # Genera muzzle_flash_fx.glb (flash_burst)
│   ├── fx_bullet_and_casing.py   # Genera bullet_projectile.glb y shell_casing_eject.glb
│   ├── arena_ruined_city.py      # Genera ruined_city_arena.glb (-col, SpawnPlayer, SpawnAI)
│   └── render_previews.py        # Renderiza 4 vistas PNG con Cycles CPU
├── rust/
│   ├── Cargo.toml                # Crate cdylib shooter_core (godot 0.5.5, api-4-7)
│   ├── rust-toolchain.toml       # Toolchain 1.100.0 + targets arm64 y armv7
│   └── src/
│       ├── lib.rs                # Registro de extensión GDExtension
│       ├── combat_math.rs        # Matemática pura de balística y puntuación táctica + unit tests
│       ├── weapon_rig.rs         # WeaponRig, TracerProjectile y EjectedCasing
│       ├── player_capsule.rs     # PlayerCapsule (movimiento, cámara OTS/FPS, agachado, retroceso)
│       └── enemy_ai_capsule.rs   # EnemyAiCapsule (FSM táctica, NavigationAgent3D, LoS, cobertura)
├── godot/
│   ├── project.godot             # Configuración Godot 4.7 Mobile + mapa de entrada
│   ├── shooter_core.gdextension  # Mapeo de libshooter_core.so (arm64, armv7 y linux host CI)
│   ├── export_presets.cfg        # Presets Android Debug separados (64 bits arm64-v8a y 32 bits armeabi-v7a)
│   ├── assets/
│   │   ├── models/               # (Ignorado en Git) Carpeta poblada desde GitHub Releases en CI
│   │   └── audio/                # Efectos de sonido CC0 (.ogg): disparos, recarga, casquillo, pasos y salto
│   ├── scenes/
│   │   ├── main.tscn             # Escena raíz que une cámaras, arena, personajes y pantallas UI
│   │   ├── arena.tscn            # NavigationRegion3D + iluminación + FxContainer
│   │   ├── player_capsule.tscn   # Nodo PlayerCapsule + SpringArm3D + WeaponRig
│   │   ├── enemy_ai_capsule.tscn # Nodo EnemyAiCapsule + NavigationAgent3D + WeaponRig
│   │   ├── ui_menu.tscn          # Menú principal + popups de Ajustes y Controles
│   │   ├── ui_hud.tscn           # HUD táctico de combate + joystick/botones multitáctiles
│   │   └── ui_end_overlay.tscn   # Pantallas de Pausa y Fin de Partida
│   └── scripts/
│       ├── game_manager.gd       # Orquestador del flujo menú -> carga -> combate/reaparición -> resultado
│       ├── asset_loader.gd       # Carga asíncrona por corrutinas de los 5 .glb
│       ├── combat_audio.gd       # Audio de combate 2D y espacial 3D (disparos, pasos, recarga, casquillo)
│       ├── arena_controller.gd   # Instancia la ciudad, hornea NavMesh y extrae coberturas
│       ├── capsule_visual_builder.gd # Crea cápsulas 3D, visor, manos y acopla el rifle + FX
│       ├── menu_screen.gd        # Lógica del menú principal y popups
│       ├── touch_hud.gd          # Control multitáctil (girar cámara disparando), mira y marcador
│       └── end_overlay.gd        # Lógica de pausa y resumen de victoria/derrota
├── src/                          # App web mínima "Hello World" (Vite + React + TypeScript)
├── README.md
├── ROADMAP.md
├── STRUCTURE.md
├── AI_CONTEXT.md
├── AGENTS.md
└── README_AI.md
```
