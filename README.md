# Iron Vector

Juego móvil 3D de disparos tácticos **1 vs 1 (Jugador vs IA Local)** construido con **Godot 4.7.2-stable** (Mobile/Vulkan Renderer), núcleo nativo en **Rust 1.100.0 (`godot = 0.5.5`, GDExtension)** y generación procedural de modelos 3D en **Blender 5.2.4** sin interfaz.

---

## Características Principales

- **Combate Táctico en Ciudad Destruida (`ruined_city_arena.glb`)**: Arena urbana de ~40m x 40m con edificios en ruinas transitables, rampas, cráteres, vehículos calcinados, trincheras de sacos de arena y barreras Jersey.
- **Movilidad Avanzada y Combate Estilo Warzone**:
  - **Slide y Slide Cancel**: Empuja el joystick izquierdo al anillo Sprint y pulsa **Agachar** para deslizarte tras coberturas; pulsar **Saltar**, **Agachar** o **Apuntar (ADS)** durante el deslizamiento ejecuta un *Slide Cancel* instantáneo recuperando impulso o postura de tiro.
  - **Lanzamiento en Plancha (Dolphin Dive)**: Botón táctico dedicado (o tecla `X`) para lanzarte hacia adelante en el aire con recuperación cuerpo a tierra.
  - **Apuntado por Mira (ADS) y Recarga en Mira**: Botón de mira óptica (o Clic Derecho / `V`) con zoom FOV dinámico, cierre de retícula, menor retroceso, animación `aim_idle` y recarga sincronizada manteniendo la mira (`aim_reload`).
  - **Granadas de Fragmentación (`frag_grenade.glb` + `grenade_explosion_fx.glb`)**: 2 granadas por ronda para el jugador (y uso táctico por parte de la IA si el jugador acampa oculto) con trayectoria parabólica, rebote físico por raycast, oclusión de metralla tras barreras y explosión 3D con audio posicional.
- **Personajes Cápsula ("Píldoras") con Rifle Clásico 7.62 (`classic_assault_rifle.glb`)**:
  - **Jugador**: HUD móvil ergonómico en arco con iconos vectoriales nativos (`TacticalButtonIcon`), disparo con arrastre simultáneo de cámara, cambio 1ª/3ª persona y respaldo completo de teclado/ratón en PC.
  - **IA Local Inteligente**: Navegación con `NavigationAgent3D` sobre `NavigationMesh` horneado al vuelo, evaluación real de línea de visión mediante raycasts 3D, tiempo de reacción humano, giro suave, ráfagas controladas de 2 a 6 disparos, uso de granadas contra enemigos cubiertos, agachado en cobertura al recargar y maniobras de flanqueo.
- **Efectos Balísticos y Explosivos en 3D**: Animaciones `idle`, `fire`, `reload`, `aim_idle`, `aim_reload` del rifle, fogonazo (`muzzle_flash_fx.glb`), trazadoras (`bullet_projectile.glb`), casquillos expulsados (`shell_casing_eject.glb`), granada giratoria (`grenade_spin`) y onda expansiva (`explode_burst`).
- **Compilación Móvil 32 y 64 bits sin PC**: Todos los `.glb` y el `.apk` Debug se compilan y empaquetan directamente desde GitHub Actions en ejecución manual (`workflow_dispatch`).

---

## Flujo de Trabajo en GitHub Actions (Sin PC)

Este repositorio **no almacena binarios `.glb` ni `.apk`** en el historial de Git (`*.glb` y `godot/assets/models/` están ignorados en `.gitignore`).

### Paso 1: Generar y Publicar Assets 3D en un Release
1. Abre la pestaña **Actions** en GitHub.
2. Selecciona **`Generate & Release Blender 3D Assets`**.
3. Pulsa **Run workflow** (solo ejecución manual) e introduce:
   - `tag_name`: por ejemplo `assets-v0.2.0`.
   - `release_title` (opcional) y `prerelease` (opcional).
4. Blender 5.2.4 generará los **7 `.glb`** y **6 capturas `.png`** de inspección y creará el **GitHub Release** con ese Tag.

### Paso 2: Compilar los APK Debug para Android (64 y 32 bits separados)
1. En la pestaña **Actions**, selecciona **`Build Android Debug APK`**.
2. Pulsa **Run workflow** (opcionalmente indica `assets_tag` o déjalo vacío para descargar el último Release publicado).
3. El flujo:
   - Descarga los `.glb` directamente desde GitHub Releases a `godot/assets/models/`.
   - Ejecuta `cargo test --lib` en Rust.
   - Compila `libshooter_core.so` en modo Debug para **64 bits (`arm64-v8a`)** y **32 bits (`armeabi-v7a`)** con `cargo-ndk` + librería host Linux para el importador de Godot.
   - Importa el proyecto con **Godot 4.7.2-stable** y exporta dos APK separados: `iron_vector-arm64-v8a-debug.apk` y `iron_vector-armeabi-v7a-debug.apk`.
4. Descarga el artefacto correspondiente a tu teléfono (**`iron-vector-android-debug-arm64-v8a`** para 64 bits o **`iron-vector-android-debug-armeabi-v7a`** para 32 bits) e instala el APK en tu Android.

---

## Entorno Web de Referencia

La raíz incluye una aplicación mínima "Hello World" en Vite + React + TypeScript (`src/`) para mantener operativa la compilación del entorno web (`npm run build`, `npm run lint`).
