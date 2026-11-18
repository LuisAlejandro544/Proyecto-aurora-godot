# Contexto Técnico del Proyecto (`AI_CONTEXT.md`)

Documento de referencia técnica para mantener consistencia arquitectónica, de coordenadas y de APIs en futuras iteraciones.

---

## 1. Stack y Versiones Exactas

| Componente | Versión / Configuración |
| :--- | :--- |
| Motor 3D | **Godot `4.7.2-stable`** (`Mobile` / Vulkan `RenderingDevice`, ETC2/ASTC) |
| Lenguaje Nativo | **Rust `1.100.0`**, edición `2024`, crate `shooter_core` (`cdylib`) |
| Bindings GDExtension | **`godot = { version = "0.5.5", features = ["api-4-7"] }`** |
| Generación 3D Headless | **Blender `5.2.4`** (scripts Python independientes en `scripts/blender/`) |
| Destino Android | APK Debug firmado (`minSdk 24`, `targetSdk 35`), ABIs `arm64-v8a` y `armeabi-v7a` |
| Entorno Web Auxiliar | Vite + React 19 + TypeScript (Hello World en `src/`) |

---

## 2. Convención Crítica de Coordenadas (Blender -> glTF -> Godot)

1. **Escenario (`ruined_city_arena.glb`)**:
   - Blender exporta con `export_yup = True`. Un punto Blender `(X, Y, Z)` se transforma en Godot `(X, Z, -Y)`.
   - `SpawnPlayer` en Blender `(0.0, -16.8, 0.1)` queda en Godot `(0.0, 0.1, +16.8)`.
   - `SpawnAI` en Blender `(0.0, +16.8, 0.1)` queda en Godot `(0.0, 0.1, -16.8)`.
   - El suelo `GroundAsphalt-col` tiene su cara superior en `Y = 0.0`.
   - Toda malla de colisión estática lleva sufijo `-col` para que Godot genere `StaticBody3D` automáticamente al importar.
2. **Rifle, Granada y Efectos (`classic_assault_rifle.glb`, `muzzle_flash_fx.glb`, `bullet_projectile.glb`, `shell_casing_eject.glb`, `frag_grenade.glb`, `grenade_explosion_fx.glb`)**:
   - En glTF el cañón del rifle y los efectos apuntan a **`+X`**, arriba es **`+Y`** y la ventana de expulsión apunta a **`+Z`**.
   - El frente estándar en Godot 4 es **`-Z`** (y la derecha es **`+X`**).
   - Por ello, `capsule_visual_builder.gd` y `weapon_rig.rs` aplican una rotación de **`+90°` (`PI / 2`) en el eje Y** al instanciar el rifle, la bala y el casquillo:
     - glTF `+X` (cañón/bala) -> Godot **`-Z`** (frente).
     - glTF `+Z` (expulsión) -> Godot **`+X`** (derecha).
   - `MuzzleFlashFX` se inserta como hijo directo de `MuzzlePoint` (que ya hereda la rotación `+90°` del rifle), por lo que apunta automáticamente hacia `-Z`.
   - Para alinear `SightPoint` (`(0.105, 0.058, 0.0)` en glTF, escalado x`1.06`) frente a la cámara en 1ª persona al apuntar ADS, `PlayerCapsule` interpola `WeaponMount` hacia `(0.0, 1.495 - drop_y, -0.34)` y reduce `fov` de `72.0` a `52.0`.

---

## 3. Política de Activos `.glb` (Sin Binarios en Git)

- Ningún archivo `.glb` o `.apk` se sube al repositorio (`*.glb` y `godot/assets/models/` están en `.gitignore`).
- Ningún `.tscn` referencia rutas `.glb` ni UIDs estáticos.
- `AssetLoader` (`godot/scripts/asset_loader.gd`) carga los **7 `.glb`** de `res://assets/models/` de forma asíncrona mediante `ResourceLoader.load_threaded_request` y corrutinas `await`.
- En CI, `android-debug-apk.yml` descarga los `.glb` desde el GitHub Release creado previamente por `blender-assets.yml`.

---

## 4. Detalles Clave de `godot-rust` (`gdext 0.5.5`) y Móvil

- **Cero re-entrancia de préstamos (`BorrowMutError`)**: Nunca emitir señales síncronas cuyo receptor en GDScript llame de vuelta a un `#[func]` sobre `&mut self` del mismo nodo Rust que aún ejecuta `physics_process` o `apply_bullet_damage`. `GameManager` conecta `"died"` con `CONNECT_DEFERRED`, y `PlayerCapsule` consulta `consume_last_hit_damage()` tras `try_fire`.
- **Protección táctil vs ratón emulado en Android**: `PlayerCapsule` solo lee acciones de ratón (`fire`, `aim`) cuando `Input.get_mouse_mode() == MouseMode::CAPTURED` para evitar disparos accidentales al deslizar en la mitad derecha de la pantalla móvil.
- Métodos con parámetros por defecto usan constructores `_ex()`:
  - `anim.play_ex().name("fire").done()`
  - `node.look_at_ex(target).up(Vector3::UP).done()`
- Para obtener `Gd<CollisionObject3D>` dentro de una clase Rust propia: usar `self.to_gd().upcast()`, nunca `self.base().clone()`.
- `self.base().get_tree_or_null()` devuelve `Option<Gd<SceneTree>>` (`get_tree()` hace panic fuera del árbol).
- `PhysicsDirectSpaceState3D::intersect_ray(&query)` devuelve `VarDictionary`; extraer campos con `result.get("key")` -> `.try_to::<T>()`.
- `SpringArm3D` expone `set_length(...)` para `spring_length`.
- `shooter_core.gdextension` mantiene habilitado `linux.debug.x86_64` para que Godot headless en Ubuntu CI reconozca `PlayerCapsule`, `EnemyAiCapsule`, `WeaponRig` y `FragGrenade` al ejecutar `--import` y `--export-debug`.
