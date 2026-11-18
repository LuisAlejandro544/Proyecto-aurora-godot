# Reglas y Directrices para Agentes (`AGENTS.md`)

Instrucciones obligatorias para cualquier agente o colaborador que modifique este proyecto.

---

## 1. Flujo de Trabajo y Restricciones del Usuario

1. **El usuario desarrolla desde un teléfono móvil (sin PC)**: todo debe compilar, probarse y empaquetarse en GitHub Actions sin depender de pasos manuales locales en escritorio.
2. **GitHub Actions estrictamente manuales (`workflow_dispatch`)**:
   - No añadir triggers `push`, `pull_request` ni `schedule` a `.github/workflows/blender-assets.yml` ni a `.github/workflows/android-debug-apk.yml`.
   - El workflow de Blender debe publicar siempre los `.glb` y `.png` en un **GitHub Release** usando el `tag_name` introducido manualmente.
   - El workflow de Android debe descargar los `.glb` desde GitHub Releases (nunca exigir `.glb` dentro del repositorio Git).
3. **Nunca subir archivos `.glb` al repositorio**: mantener `*.glb` y `godot/assets/models/` en `.gitignore`.
4. **Arquitecturas Android obligatorias**: compilar y empaquetar siempre **64 bits (`arm64-v8a`)** y **32 bits (`armeabi-v7a`)** en el APK Debug.
5. **Licencias y Marcas**:
   - No usar nombres de marcas comerciales registradas en nombres de archivos, clases ni strings visibles (usar "Iron Vector", "rifle clásico 7.62", etc.).
   - No recomendar ni agregar dependencias con licencias copyleft que obliguen a abrir el código fuente o exigir créditos obligatorios (priorizar MIT / Apache-2.0).

---

## 2. Reglas de Arquitectura y Diseño

- **Desarrollo Modular**: separar lógica pura (`combat_math.rs`), armas (`weapon_rig.rs`), jugador (`player_capsule.rs`), IA (`enemy_ai_capsule.rs`), carga de assets (`asset_loader.gd`), construcción de cápsulas (`capsule_visual_builder.gd`), arena (`arena_controller.gd`) y pantallas UI.
- **Uso de corrutinas / hilos secundarios**: en móvil, no bloquear el hilo principal al cargar `.glb` (`ResourceLoader.load_threaded_request`) ni al hornear mallas de navegación (`bake_navigation_mesh(true)`).
- **UI con pantallas separadas y sin minimalismo extremo**: mantener Menú Principal, Popups de Ajustes/Controles, Carga, HUD de combate y Pausa/Resultados como escenas separadas con identidad visual táctica clara.
- **No modificar diseño ni lógica funcional existente** sin que el usuario lo solicite explícitamente.
- **Comentarios explicativos**: todo archivo de código nuevo o editado debe explicar claramente la responsabilidad de su lógica.
- **No tocar `persist.sys.*`** en ningún script o código móvil.
