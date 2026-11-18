# Roadmap de Desarrollo — Iron Vector

Hoja de ruta evolutiva desde la base tecnológica hasta un shooter táctico móvil completo.

---

## Fase 1 — Infraestructura y Pipeline Sin PC (Completada)
- [x] Configuración de Godot **4.7.2-stable** (Mobile Renderer, ETC2/ASTC) y **Rust 1.100.0** (`godot = 0.5.5`, `api-4-7`).
- [x] Soporte simultáneo para arquitecturas Android **64 bits (`arm64-v8a`)** y **32 bits (`armeabi-v7a`)**, Android SDK min 24 / target 35.
- [x] Entorno web mínimo ("Hello World") funcional.

## Fase 2 — Activos 3D Procedurales en Blender Headless (Completada)
- [x] Scripts Python modulares en `scripts/blender/` para Blender 5.2.4 sin interfaz.
- [x] Rifle clásico de asalto 7.62 con `MuzzlePoint`, `EjectPoint`, `SightPoint` y animaciones `idle`, `fire`, `reload`.
- [x] Fogonazo (`flash_burst`), proyectil trazador y casquillo con trayectoria parabólica y rebote (`casing_eject`).
- [x] Ciudad destruida con edificios, rampas, barreras, autos calcinados, sacos de arena y nodos `SpawnPlayer` / `SpawnAI`.
- [x] Workflow manual `blender-assets.yml` que publica los `.glb` y `.png` en GitHub Releases con Tag elegido por el usuario.

## Fase 3 — Jugabilidad Base: Cápsulas vs IA Táctica Local (Completada)
- [x] Módulo puro `combat_math.rs` con pruebas unitarias de caída de daño, dispersión, ráfagas y puntuación de coberturas.
- [x] Controlador `WeaponRig` en Rust con recarga, retroceso, fogonazo, trazadoras, casquillos y raycast de doble etapa.
- [x] Controlador `PlayerCapsule` con agachado físico real para cubrirse, cámara 3ª/1ª persona y controles multitáctiles.
- [x] Controlador `EnemyAiCapsule` con `NavigationAgent3D`, verificación real de línea de tiro en coberturas, ráfagas humanas y flanqueo.
- [x] Pantallas separadas: Menú Principal (con popups de Ajustes y Controles), Carga Asíncrona por corrutinas, HUD Táctico y Pausa/Resultados.

## Fase 4 — Audio, Feedback e Impacto (Próxima)
- [ ] Integración de efectos de sonido procedurales/Ogg: disparos 7.62, sonido metálico de casquillos, recarga, impactos en concreto y cuerpo.
- [ ] Partículas de impacto de bala (chispas en metal/vehículos y polvo en hormigón/sacos de arena).
- [ ] Indicador direccional de daño en el HUD cuando la IA dispara desde un flanco.

## Fase 5 — Personajes y Variedad Táctica (Futura)
- [ ] Reemplazo de la malla cápsula por personajes bípedos estilizados en Blender con esqueleto (`Rig`) y animaciones de correr, apuntar y agacharse.
- [ ] Segunda arma secundaria (pistola táctica o subfusil) y granadas de humo/fragmentación.
- [ ] Segundo escenario urbano nocturno o zona industrial y modo de oleadas contra múltiples bots coordinados.
