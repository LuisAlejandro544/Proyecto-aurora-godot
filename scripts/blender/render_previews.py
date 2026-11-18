"""
Script independiente de Blender (sin interfaz) para renderizar vistas previas en PNG
usando Cycles CPU. Permite revisar visualmente desde el teléfono o entorno web:
1. El rifle clásico 7.62 completo (vista lateral/tres cuartos).
2. La animación de recarga del rifle (cargador curvo saliendo y cerrojo atrás).
3. El efecto de fogonazo + bala trazadora + casquillos expulsados cayendo al suelo.
4. Vista aérea isométrica de la ciudad destruida y sus coberturas.
5. Vista a ras de calle mostrando cobertura táctica y edificios en ruinas.
"""

import math
import os
import sys
import bpy
from mathutils import Euler, Vector

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from arena_ruined_city import build_ruined_city
from fx_bullet_and_casing import _create_lathe_mesh
from fx_muzzle_flash import _create_flash_cone_fin, _create_side_wings
from scene_utils import (
    clear_scene,
    create_box,
    create_empty,
    get_or_create_collection,
    make_pbr_material,
)
from weapon_classic_rifle import _build_rifle_geometry, _create_animations, build_classic_rifle
from weapon_frag_grenade import build_frag_grenade


def _setup_cycles_render(res_x=1080, res_y=640, samples=24, bg_color=(0.14, 0.16, 0.20, 1.0)):
    """Configura Cycles en CPU con denoiser para previews limpias y rápidas."""
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = samples
    scene.cycles.use_adaptive_sampling = True
    if hasattr(scene.cycles, "use_denoising"):
        scene.cycles.use_denoising = True
    scene.render.resolution_x = res_x
    scene.render.resolution_y = res_y
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"

    if scene.world is None:
        scene.world = bpy.data.worlds.new("PreviewWorld")
    if hasattr(scene.world, "use_nodes") and not scene.world.use_nodes:
        scene.world.use_nodes = True
    bg = scene.world.node_tree.nodes.get("Background") if scene.world.node_tree else None
    if bg:
        bg.inputs["Color"].default_value = bg_color
        bg.inputs["Strength"].default_value = 0.85


def _add_studio_lights():
    """Añade iluminación de estudio de 3 puntos para armas y efectos."""
    key_data = bpy.data.lights.new("KeyLight", type="AREA")
    key_data.energy = 125.0
    key_data.size = 2.0
    key = bpy.data.objects.new("KeyLight", key_data)
    key.location = (1.6, -1.8, 1.8)
    bpy.context.scene.collection.objects.link(key)

    fill_data = bpy.data.lights.new("FillLight", type="AREA")
    fill_data.energy = 60.0
    fill_data.size = 2.5
    fill = bpy.data.objects.new("FillLight", fill_data)
    fill.location = (-1.4, -1.6, 1.0)
    bpy.context.scene.collection.objects.link(fill)

    rim_data = bpy.data.lights.new("RimLight", type="AREA")
    rim_data.energy = 95.0
    rim_data.size = 1.8
    rim = bpy.data.objects.new("RimLight", rim_data)
    rim.location = (0.4, 1.8, 1.4)
    bpy.context.scene.collection.objects.link(rim)

    for l_obj in (key, fill, rim):
        direction = Vector((0.05, 0.0, 0.0)) - l_obj.location
        l_obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def _add_sun_light():
    """Añade luz solar de tarde para resaltar sombras de coberturas en la ciudad."""
    sun_data = bpy.data.lights.new("CitySun", type="SUN")
    sun_data.energy = 3.4
    sun_data.angle = math.radians(3.0)
    sun = bpy.data.objects.new("CitySun", sun_data)
    sun.location = (18.0, -14.0, 24.0)
    direction = Vector((0.0, 0.0, 0.0)) - sun.location
    sun.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.collection.objects.link(sun)


def _point_camera(name, location, target, lens=40.0):
    """Crea y apunta una cámara activa hacia un punto objetivo."""
    cam_data = bpy.data.cameras.new(name)
    cam_data.lens = lens
    cam_data.clip_end = 200.0
    cam = bpy.data.objects.new(name, cam_data)
    cam.location = Vector(location)
    direction = Vector(target) - cam.location
    cam.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.collection.objects.link(cam)
    bpy.context.scene.camera = cam
    return cam


def _render_to(filepath):
    """Ejecuta el render de la escena actual y lo guarda en disco."""
    os.makedirs(os.path.dirname(os.path.abspath(filepath)), exist_ok=True)
    bpy.context.scene.render.filepath = os.path.abspath(filepath)
    bpy.ops.render.render(write_still=True)
    print(f"[Preview Render] PNG generado: {filepath}")


def _set_active_nla_only(target_track_name):
    """Activa únicamente los NLA strips cuyo nombre coincide para previsualizar una acción concreta."""
    for obj in bpy.data.objects:
        if obj.animation_data and obj.animation_data.nla_tracks:
            for tr in obj.animation_data.nla_tracks:
                tr.mute = tr.name != target_track_name


def _spawn_preview_fx(col):
    """Instancia directamente fogonazo, bala trazadora y secuencia de casquillos cayendo."""
    # Fogonazo en boca del cañón (MuzzlePoint = 0.568, 0.0, 0.012)
    mat_core = make_pbr_material("Prev_FlashCore", (1.0, 0.96, 0.72, 1.0), emission_color=(1.0, 0.92, 0.55, 1.0), emission_strength=16.0)
    mat_outer = make_pbr_material("Prev_FlashOuter", (1.0, 0.42, 0.05, 0.85), emission_color=(1.0, 0.42, 0.05, 1.0), emission_strength=9.0, alpha=0.85)
    flash_root = create_empty("PrevFlash", location=(0.555, 0.0, 0.012), collection=col)
    _create_flash_cone_fin("PrevFlashCore", 0.145, 0.042, 6, mat_core, flash_root, col)
    _create_flash_cone_fin("PrevFlashOuter", 0.235, 0.075, 8, mat_outer, flash_root, col)
    _create_side_wings("PrevFlashWings", mat_outer, flash_root, col)

    # Luz cálida emanada del disparo sobre el arma y el suelo
    flash_light_data = bpy.data.lights.new("MuzzleGlowLight", type="POINT")
    flash_light_data.energy = 38.0
    flash_light_data.color = (1.0, 0.65, 0.25)
    flash_light = bpy.data.objects.new("MuzzleGlowLight", flash_light_data)
    flash_light.location = (0.62, -0.08, 0.02)
    bpy.context.scene.collection.objects.link(flash_light)

    # Bala 7.62 trazadora saliendo del fogonazo
    mat_copper = make_pbr_material("Prev_Copper", (0.85, 0.46, 0.18, 1.0), metallic=0.92, roughness=0.22)
    mat_tracer = make_pbr_material("Prev_Tracer", (1.0, 0.52, 0.08, 1.0), emission_color=(1.0, 0.48, 0.08, 1.0), emission_strength=12.0)
    bullet_root = create_empty("PrevBullet", location=(0.88, 0.0, 0.012), collection=col)
    bullet_root.scale = (2.6, 2.6, 2.6)
    bullet_profile = [
        (-0.015, 0.0), (-0.015, 0.0042), (-0.008, 0.0052),
        (0.006, 0.0052), (0.018, 0.0038), (0.028, 0.0014), (0.032, 0.0),
    ]
    _create_lathe_mesh("PrevBulletMesh", bullet_profile, 14, mat_copper, bullet_root, col)
    _create_lathe_mesh("PrevBulletTracer", [(-0.028, 0.0), (-0.022, 0.0035), (-0.014, 0.0035), (-0.014, 0.0)], 12, mat_tracer, bullet_root, col)

    # 3 casquillos mostrando la trayectoria: expulsión -> caída -> suelo
    mat_brass = make_pbr_material("Prev_Brass", (0.90, 0.72, 0.25, 1.0), metallic=0.95, roughness=0.24)
    casing_profile = [
        (-0.028, 0.0), (-0.028, 0.0085), (-0.025, 0.0072), (-0.023, 0.0085),
        (0.014, 0.0075), (0.020, 0.0062), (0.028, 0.0062), (0.028, 0.0052),
        (0.018, 0.0052), (0.018, 0.0),
    ]
    casing_poses = [
        ("Ejecting", (0.02, -0.14, 0.14), (35.0, 50.0, 22.0)),
        ("Falling", (-0.06, -0.32, -0.14), (80.0, 125.0, 45.0)),
        ("Ground1", (-0.12, -0.44, -0.525), (4.0, 8.0, 38.0)),
        ("Ground2", (-0.22, -0.52, -0.525), (6.0, -5.0, 112.0)),
    ]
    for tag, loc, rot_deg in casing_poses:
        c_root = create_empty(f"PrevCasing_{tag}", location=loc, collection=col)
        c_root.rotation_euler = Euler((math.radians(rot_deg[0]), math.radians(rot_deg[1]), math.radians(rot_deg[2])), "XYZ")
        c_root.scale = (2.6, 2.6, 2.6)
        _create_lathe_mesh(f"PrevCasingMesh_{tag}", casing_profile, 14, mat_brass, c_root, col)


def render_all_previews(out_dir="previews"):
    """Genera todas las capturas PNG de verificación para el arma, efectos y ciudad."""
    # 1. Rifle clásico completo en pose lista
    build_classic_rifle(output_glb=None)
    _setup_cycles_render(res_x=1080, res_y=620, samples=28)
    _add_studio_lights()
    _set_active_nla_only("idle")
    bpy.context.scene.frame_set(1)
    _point_camera("CamRifle", (0.58, -1.24, 0.24), (0.12, 0.0, -0.06), lens=42.0)
    _render_to(os.path.join(out_dir, "01_classic_rifle_overview.png"))

    # 2. Rifle clásico en plena recarga (frame 19)
    _set_active_nla_only("reload")
    bpy.context.scene.frame_set(19)
    _point_camera("CamReload", (0.52, -1.18, 0.18), (0.08, 0.0, -0.10), lens=40.0)
    _render_to(os.path.join(out_dir, "02_classic_rifle_reload.png"))

    # 3. Fogonazo + bala + casquillos cayendo al suelo
    clear_scene()
    _setup_cycles_render(res_x=1080, res_y=620, samples=28, bg_color=(0.08, 0.09, 0.11, 1.0))
    _add_studio_lights()
    col_fx = get_or_create_collection("PreviewFX")
    mat_floor = make_pbr_material("FX_Floor", (0.15, 0.16, 0.18, 1.0), roughness=0.78)
    create_box("FXFloor", size=(4.5, 4.5, 0.05), location=(0.15, -0.35, -0.57), material=mat_floor, collection=col_fx)

    rifle_root, bolt, mag = _build_rifle_geometry(col_fx)
    _create_animations(rifle_root, bolt, mag)
    _set_active_nla_only("fire")
    _spawn_preview_fx(col_fx)

    bpy.context.scene.frame_set(2)
    _point_camera("CamFX", (0.62, -1.52, 0.12), (0.24, -0.12, -0.18), lens=38.0)
    _render_to(os.path.join(out_dir, "03_fx_muzzle_bullet_casing.png"))

    # 4. Ciudad destruida — Vista aérea isométrica general
    build_ruined_city(output_glb=None)
    _setup_cycles_render(res_x=1200, res_y=760, samples=22, bg_color=(0.18, 0.21, 0.26, 1.0))
    _add_sun_light()
    _point_camera("CamCityTop", (26.0, -28.0, 24.0), (0.0, 0.0, 1.2), lens=32.0)
    _render_to(os.path.join(out_dir, "04_ruined_city_overview.png"))

    # 5. Ciudad destruida — Vista a nivel de calle (altura de jugador ~1.7m)
    _point_camera("CamCityStreet", (-1.8, -15.2, 1.75), (1.0, 4.0, 1.4), lens=28.0)
    _render_to(os.path.join(out_dir, "05_ruined_city_street_cover.png"))

    # 6. Granada de fragmentación táctica
    build_frag_grenade(output_glb=None)
    _setup_cycles_render(res_x=880, res_y=620, samples=26, bg_color=(0.11, 0.13, 0.16, 1.0))
    _add_studio_lights()
    _point_camera("CamGrenade", (0.26, -0.34, 0.14), (0.01, 0.0, 0.02), lens=45.0)
    _render_to(os.path.join(out_dir, "06_frag_grenade_overview.png"))


if __name__ == "__main__":
    out_folder = sys.argv[-1] if len(sys.argv) > 1 and not sys.argv[-1].endswith(".py") else "previews"
    render_all_previews(out_folder)
