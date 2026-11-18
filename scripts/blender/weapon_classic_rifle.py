"""
Generador procedural en Blender (sin interfaz) del rifle clásico de asalto 7.62.
Modela piezas separadas para animación realista (cuerpo principal, cerrojo móvil,
cargador curvo desmontable con costillas, culata de madera laminada oscura,
guardamanos, tubo de gases, mira delantera con orejas protectoras y freno de boca).

Incluye animaciones exportadas en GLB:
- 'idle'   : estabilidad sutil al apuntar.
- 'fire'   : retroceso del arma + retroceso rápido del cerrojo.
- 'reload' : inclinación táctica, extracción del cargador curvo, inserción y carga de cerrojo.

Puntos de anclaje (Empties) incluidos:
- MuzzlePoint : boca del freno de boca (para fogonazo y origen de disparo).
- EjectPoint  : ventana de expulsión derecha (para casquillos saliendo).
- SightPoint  : alineación de mira trasera/delantera.
"""

import math
import os
import sys
import bpy
import bmesh
from mathutils import Euler

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from scene_utils import (
    add_bevel,
    assign_material,
    clear_scene,
    create_box,
    create_cylinder,
    create_empty,
    export_glb,
    get_or_create_collection,
    link_to_collection,
    make_pbr_material,
    push_action_to_nla,
)


def _create_curved_magazine(name, mat_steel, parent, collection):
    """Construye el icónico cargador curvo en arco (30 balas 7.62) con base limpia y costillas."""
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()

    steps = 12
    arc_radius = 0.34
    start_angle = 0.0
    end_angle = math.radians(33.0)
    width_y = 0.025
    depth_x = 0.066

    rings = []
    for i in range(steps + 1):
        t = i / steps
        ang = start_angle + (end_angle - start_angle) * t
        cx = arc_radius * (1.0 - math.cos(ang))
        cz = -arc_radius * math.sin(ang)

        nx = math.cos(ang)
        nz = -math.sin(ang)

        # Pequeña costilla de refuerzo en pasos alternos
        rib_y = 0.0022 if (i in (3, 6, 9)) else 0.0
        hx = depth_x * 0.5 * (1.0 - 0.04 * t)
        hy = width_y * 0.5 + rib_y
        v0 = bm.verts.new((cx - hx * nx, -hy, cz - hx * nz))
        v1 = bm.verts.new((cx + hx * nx, -hy, cz + hx * nz))
        v2 = bm.verts.new((cx + hx * nx, hy, cz + hx * nz))
        v3 = bm.verts.new((cx - hx * nx, hy, cz - hx * nz))
        rings.append([v0, v1, v2, v3])

    for i in range(len(rings) - 1):
        r0 = rings[i]
        r1 = rings[i + 1]
        for j in range(4):
            jn = (j + 1) % 4
            bm.faces.new((r0[j], r0[jn], r1[jn], r1[j]))

    bm.faces.new(rings[0][::-1])
    bm.faces.new(rings[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    obj.location = (0.065, 0.0, -0.022)
    bpy.context.scene.collection.objects.link(obj)
    link_to_collection(obj, collection)
    assign_material(obj, mat_steel)
    add_bevel(obj, width=0.0018, segments=2)
    obj.parent = parent

    # Base metálica inferior alineada exactamente al último segmento del arco
    ang_end = end_angle
    bx = arc_radius * (1.0 - math.cos(ang_end))
    bz = -arc_radius * math.sin(ang_end)
    floorplate = create_box(
        "MagFloorplate",
        size=(0.072, 0.030, 0.010),
        location=(bx, 0.0, bz - 0.004),
        rotation=(0.0, ang_end, 0.0),
        material=mat_steel,
        bevel_width=0.0015,
        collection=collection,
    )
    floorplate.parent = obj
    return obj


def _create_wooden_buttstock(name, mat_wood, mat_steel, parent, collection):
    """Construye la culata clásica de madera laminada inclinada hacia abajo con cantonera."""
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()

    profile = [
        (-0.120, 0.022),
        (-0.120, -0.022),
        (-0.345, -0.074),
        (-0.356, -0.074),
        (-0.356, 0.006),
        (-0.235, 0.016),
    ]
    half_y = 0.017
    front_verts = [bm.verts.new((x, -half_y, z)) for (x, z) in profile]
    back_verts = [bm.verts.new((x, half_y, z)) for (x, z) in profile]

    bm.faces.new(front_verts)
    bm.faces.new(back_verts[::-1])
    n = len(profile)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((front_verts[i], front_verts[j], back_verts[j], back_verts[i]))

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()

    stock = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(stock)
    link_to_collection(stock, collection)
    assign_material(stock, mat_wood)
    add_bevel(stock, width=0.005, segments=2)
    stock.parent = parent

    buttplate = create_box(
        "Buttplate",
        size=(0.012, 0.037, 0.086),
        location=(-0.358, 0.0, -0.034),
        rotation=(0.0, math.radians(-5.0), 0.0),
        material=mat_steel,
        bevel_width=0.002,
        collection=collection,
    )
    buttplate.parent = stock
    return stock


def _create_pistol_grip(name, mat_wood, parent, collection):
    """Construye la empuñadura trasera inclinada y ergonómica."""
    grip = create_box(
        name,
        size=(0.034, 0.027, 0.105),
        location=(-0.068, 0.0, -0.068),
        rotation=(0.0, math.radians(24.0), 0.0),
        material=mat_wood,
        bevel_width=0.004,
        collection=collection,
    )
    grip.parent = parent
    return grip


def _build_rifle_geometry(collection):
    """Construye toda la geometría del rifle clásico y devuelve los objetos clave para animar."""
    mat_steel_dark = make_pbr_material(
        "Rifle_Steel_Dark",
        base_color=(0.065, 0.070, 0.076, 1.0),
        metallic=0.88,
        roughness=0.33,
    )
    mat_steel_bolt = make_pbr_material(
        "Rifle_Steel_Bolt",
        base_color=(0.50, 0.51, 0.53, 1.0),
        metallic=0.95,
        roughness=0.20,
    )
    # Madera laminada cálida oscura (tono clásico caoba/nogal)
    mat_wood_laminate = make_pbr_material(
        "Rifle_Wood_Laminate",
        base_color=(0.24, 0.078, 0.025, 1.0),
        metallic=0.02,
        roughness=0.38,
    )
    mat_mag_steel = make_pbr_material(
        "Rifle_Mag_Steel",
        base_color=(0.10, 0.105, 0.112, 1.0),
        metallic=0.84,
        roughness=0.36,
    )
    mat_sight_dot = make_pbr_material(
        "Rifle_Sight_Glow",
        base_color=(0.2, 0.95, 0.35, 1.0),
        metallic=0.0,
        roughness=0.25,
        emission_color=(0.2, 0.95, 0.35, 1.0),
        emission_strength=1.8,
    )

    root = create_empty("ClassicRifleRoot", location=(0.0, 0.0, 0.0), collection=collection)

    # 1. Cajón de mecanismos (Stamped Receiver)
    receiver = create_box(
        "Receiver",
        size=(0.24, 0.036, 0.056),
        location=(0.0, 0.0, 0.0),
        material=mat_steel_dark,
        bevel_width=0.0025,
        collection=collection,
    )
    receiver.parent = root

    dust_cover = create_box(
        "DustCover",
        size=(0.215, 0.032, 0.024),
        location=(-0.01, 0.0, 0.034),
        material=mat_steel_dark,
        bevel_width=0.004,
        collection=collection,
    )
    dust_cover.parent = root

    # Guardamonte abierto (aro inferior + frente) y gatillo curvo
    tg_bottom = create_box(
        "TriggerGuard_Bottom",
        size=(0.044, 0.012, 0.004),
        location=(-0.024, 0.0, -0.050),
        material=mat_steel_dark,
        bevel_width=0.001,
        collection=collection,
    )
    tg_bottom.parent = root

    tg_front = create_box(
        "TriggerGuard_Front",
        size=(0.005, 0.012, 0.024),
        location=(-0.003, 0.0, -0.038),
        rotation=(0.0, math.radians(-12.0), 0.0),
        material=mat_steel_dark,
        bevel_width=0.001,
        collection=collection,
    )
    tg_front.parent = root

    trigger = create_box(
        "Trigger",
        size=(0.007, 0.005, 0.020),
        location=(-0.024, 0.0, -0.036),
        rotation=(0.0, math.radians(-18.0), 0.0),
        material=mat_steel_bolt,
        bevel_width=0.001,
        collection=collection,
    )
    trigger.parent = root

    # 2. Cerrojo y palanca de carga móvil (Bolt Carrier) en costado derecho (-Y)
    bolt_carrier = create_box(
        "BoltCarrier",
        size=(0.088, 0.010, 0.018),
        location=(0.048, -0.017, 0.022),
        material=mat_steel_bolt,
        bevel_width=0.0018,
        collection=collection,
    )
    bolt_carrier.parent = root

    charging_handle = create_box(
        "ChargingHandle",
        size=(0.018, 0.024, 0.009),
        location=(0.012, -0.014, -0.002),
        rotation=(math.radians(15.0), 0.0, 0.0),
        material=mat_steel_bolt,
        bevel_width=0.002,
        collection=collection,
    )
    charging_handle.parent = bolt_carrier

    # 3. Culata y empuñadura de madera laminada
    _create_wooden_buttstock("ButtStock", mat_wood_laminate, mat_steel_dark, root, collection)
    _create_pistol_grip("PistolGrip", mat_wood_laminate, root, collection)

    # 4. Guardamanos inferior y superior de madera
    handguard_lower = create_box(
        "HandguardLower",
        size=(0.175, 0.038, 0.034),
        location=(0.208, 0.0, 0.002),
        material=mat_wood_laminate,
        bevel_width=0.006,
        collection=collection,
    )
    handguard_lower.parent = root

    handguard_upper = create_box(
        "HandguardUpper",
        size=(0.165, 0.032, 0.024),
        location=(0.205, 0.0, 0.032),
        material=mat_wood_laminate,
        bevel_width=0.005,
        collection=collection,
    )
    handguard_upper.parent = root

    retainer = create_box(
        "HandguardRetainer",
        size=(0.014, 0.040, 0.044),
        location=(0.296, 0.0, 0.016),
        material=mat_steel_dark,
        bevel_width=0.002,
        collection=collection,
    )
    retainer.parent = root

    # 5. Cañón, tubo de gases y bloque de gases clásico a 45°
    barrel = create_cylinder(
        "Barrel",
        radius=0.0095,
        depth=0.415,
        segments=16,
        location=(0.325, 0.0, 0.012),
        rotation=(0.0, math.radians(90.0), 0.0),
        material=mat_steel_dark,
        collection=collection,
    )
    barrel.parent = root

    gas_tube = create_cylinder(
        "GasTube",
        radius=0.0085,
        depth=0.220,
        segments=14,
        location=(0.228, 0.0, 0.036),
        rotation=(0.0, math.radians(90.0), 0.0),
        material=mat_steel_dark,
        collection=collection,
    )
    gas_tube.parent = root

    # Bloque de gases estilizado conectando cañón con el tubo superior
    gas_block = create_box(
        "GasBlock",
        size=(0.024, 0.019, 0.036),
        location=(0.342, 0.0, 0.024),
        rotation=(0.0, math.radians(-35.0), 0.0),
        material=mat_steel_dark,
        bevel_width=0.002,
        collection=collection,
    )
    gas_block.parent = root

    cleaning_rod = create_cylinder(
        "CleaningRod",
        radius=0.0030,
        depth=0.350,
        segments=10,
        location=(0.320, 0.0, -0.005),
        rotation=(0.0, math.radians(90.0), 0.0),
        material=mat_steel_bolt,
        collection=collection,
    )
    cleaning_rod.parent = root

    muzzle_brake = create_cylinder(
        "MuzzleBrake",
        radius=0.012,
        depth=0.038,
        segments=16,
        location=(0.545, 0.0, 0.012),
        rotation=(0.0, math.radians(90.0), 0.0),
        material=mat_steel_dark,
        collection=collection,
    )
    muzzle_brake.parent = root

    # 6. Miras (trasera tangente y poste delantero con orejas protectoras)
    rear_sight_base = create_box(
        "RearSightBase",
        size=(0.044, 0.024, 0.018),
        location=(0.105, 0.0, 0.044),
        material=mat_steel_dark,
        bevel_width=0.0015,
        collection=collection,
    )
    rear_sight_base.parent = root

    front_sight_base = create_box(
        "FrontSightBase",
        size=(0.024, 0.017, 0.042),
        location=(0.495, 0.0, 0.028),
        material=mat_steel_dark,
        bevel_width=0.0015,
        collection=collection,
    )
    front_sight_base.parent = root

    front_sight_hood_l = create_box(
        "FrontSightEar_L",
        size=(0.011, 0.003, 0.022),
        location=(0.495, 0.007, 0.056),
        material=mat_steel_dark,
        collection=collection,
    )
    front_sight_hood_l.parent = root

    front_sight_hood_r = create_box(
        "FrontSightEar_R",
        size=(0.011, 0.003, 0.022),
        location=(0.495, -0.007, 0.056),
        material=mat_steel_dark,
        collection=collection,
    )
    front_sight_hood_r.parent = root

    front_sight_post = create_cylinder(
        "FrontSightPost",
        radius=0.0016,
        depth=0.016,
        segments=8,
        location=(0.495, 0.0, 0.054),
        material=mat_sight_dot,
        collection=collection,
    )
    front_sight_post.parent = root

    # 7. Cargador curvo clásico
    magazine = _create_curved_magazine("Magazine", mat_mag_steel, root, collection)

    # 8. Puntos de anclaje (Empties) para Godot/Rust
    create_empty("MuzzlePoint", location=(0.568, 0.0, 0.012), parent=root, collection=collection)
    create_empty("EjectPoint", location=(0.045, -0.024, 0.024), parent=root, collection=collection)
    create_empty("SightPoint", location=(0.105, 0.0, 0.058), parent=root, collection=collection)
    create_empty("GripPoint", location=(-0.068, 0.0, -0.050), parent=root, collection=collection)

    return root, bolt_carrier, magazine


def _keyframe_loc_rot(obj, frame, loc=None, rot_deg=None):
    """Inserta keyframes de posición y/o rotación en el frame indicado."""
    if loc is not None:
        obj.location = loc
        obj.keyframe_insert(data_path="location", frame=frame)
    if rot_deg is not None:
        obj.rotation_euler = Euler(
            (math.radians(rot_deg[0]), math.radians(rot_deg[1]), math.radians(rot_deg[2])),
            "XYZ",
        )
        obj.keyframe_insert(data_path="rotation_euler", frame=frame)


def _create_animations(root, bolt_carrier, magazine):
    """Genera las animaciones principales (idle, fire, reload, aim_idle, aim_reload) en NLA tracks independientes."""
    base_bolt_loc = tuple(bolt_carrier.location)
    base_mag_loc = tuple(magazine.location)

    # --- 1. IDLE (frames 1..40) ---
    act_root_idle = bpy.data.actions.new("idle")
    root.animation_data_create()
    root.animation_data.action = act_root_idle
    _keyframe_loc_rot(root, 1, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    _keyframe_loc_rot(root, 20, (0.0, 0.0, 0.004), (0.4, -0.6, 0.2))
    _keyframe_loc_rot(root, 40, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    push_action_to_nla(root, act_root_idle, "idle", start_frame=1)

    # --- 2. FIRE (frames 1..10): Retroceso del arma + cerrojo hacia atrás ---
    act_root_fire = bpy.data.actions.new("fire")
    root.animation_data.action = act_root_fire
    _keyframe_loc_rot(root, 1, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    _keyframe_loc_rot(root, 3, (-0.045, 0.0, 0.012), (0.0, -5.5, 0.0))
    _keyframe_loc_rot(root, 6, (-0.020, 0.0, 0.005), (0.0, -2.0, 0.0))
    _keyframe_loc_rot(root, 10, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    push_action_to_nla(root, act_root_fire, "fire", start_frame=1)

    act_bolt_fire = bpy.data.actions.new("fire_bolt")
    bolt_carrier.animation_data_create()
    bolt_carrier.animation_data.action = act_bolt_fire
    _keyframe_loc_rot(bolt_carrier, 1, base_bolt_loc)
    _keyframe_loc_rot(bolt_carrier, 2, (base_bolt_loc[0] - 0.085, base_bolt_loc[1], base_bolt_loc[2]))
    _keyframe_loc_rot(bolt_carrier, 5, base_bolt_loc)
    push_action_to_nla(bolt_carrier, act_bolt_fire, "fire", start_frame=1)

    # --- 3. RELOAD (frames 1..70): Inclinación, retirada de cargador, inserción y cerrojo ---
    act_root_reload = bpy.data.actions.new("reload")
    root.animation_data.action = act_root_reload
    _keyframe_loc_rot(root, 1, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    _keyframe_loc_rot(root, 12, (-0.02, 0.01, -0.02), (-18.0, 10.0, 12.0))
    _keyframe_loc_rot(root, 42, (-0.02, 0.01, -0.02), (-16.0, 8.0, 10.0))
    _keyframe_loc_rot(root, 55, (-0.035, 0.0, -0.01), (-12.0, 4.0, 6.0))
    _keyframe_loc_rot(root, 70, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    push_action_to_nla(root, act_root_reload, "reload", start_frame=1)

    act_mag_reload = bpy.data.actions.new("reload_mag")
    magazine.animation_data_create()
    magazine.animation_data.action = act_mag_reload
    _keyframe_loc_rot(magazine, 1, base_mag_loc, (0.0, 0.0, 0.0))
    _keyframe_loc_rot(magazine, 10, base_mag_loc, (0.0, 0.0, 0.0))
    _keyframe_loc_rot(magazine, 18, (base_mag_loc[0] + 0.025, 0.0, base_mag_loc[2] - 0.045), (0.0, -24.0, 0.0))
    _keyframe_loc_rot(magazine, 28, (base_mag_loc[0] + 0.055, 0.0, base_mag_loc[2] - 0.30), (0.0, -30.0, 0.0))
    _keyframe_loc_rot(magazine, 36, (base_mag_loc[0] + 0.025, 0.0, base_mag_loc[2] - 0.045), (0.0, -22.0, 0.0))
    _keyframe_loc_rot(magazine, 44, base_mag_loc, (0.0, 0.0, 0.0))
    _keyframe_loc_rot(magazine, 70, base_mag_loc, (0.0, 0.0, 0.0))
    push_action_to_nla(magazine, act_mag_reload, "reload", start_frame=1)

    act_bolt_reload = bpy.data.actions.new("reload_bolt")
    bolt_carrier.animation_data.action = act_bolt_reload
    _keyframe_loc_rot(bolt_carrier, 1, base_bolt_loc)
    _keyframe_loc_rot(bolt_carrier, 48, base_bolt_loc)
    _keyframe_loc_rot(bolt_carrier, 55, (base_bolt_loc[0] - 0.090, base_bolt_loc[1], base_bolt_loc[2]))
    _keyframe_loc_rot(bolt_carrier, 60, base_bolt_loc)
    _keyframe_loc_rot(bolt_carrier, 70, base_bolt_loc)
    push_action_to_nla(bolt_carrier, act_bolt_reload, "reload", start_frame=1)

    # --- 4. AIM_IDLE (frames 1..40): Estabilidad firme al apuntar por la mira ---
    act_root_aim_idle = bpy.data.actions.new("aim_idle")
    root.animation_data.action = act_root_aim_idle
    _keyframe_loc_rot(root, 1, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    _keyframe_loc_rot(root, 20, (0.0, 0.0, 0.0012), (0.0, -0.15, 0.0))
    _keyframe_loc_rot(root, 40, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    push_action_to_nla(root, act_root_aim_idle, "aim_idle", start_frame=1)

    # --- 5. AIM_RELOAD (frames 1..66): Recarga manteniendo la mira alineada al frente ---
    act_root_aim_reload = bpy.data.actions.new("aim_reload")
    root.animation_data.action = act_root_aim_reload
    _keyframe_loc_rot(root, 1, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    _keyframe_loc_rot(root, 10, (-0.012, 0.004, -0.006), (-5.5, 2.5, 3.0))
    _keyframe_loc_rot(root, 40, (-0.012, 0.004, -0.006), (-4.5, 2.0, 2.5))
    _keyframe_loc_rot(root, 52, (-0.022, 0.0, -0.004), (-3.0, 1.5, 1.5))
    _keyframe_loc_rot(root, 66, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    push_action_to_nla(root, act_root_aim_reload, "aim_reload", start_frame=1)

    act_mag_aim_reload = bpy.data.actions.new("aim_reload_mag")
    magazine.animation_data.action = act_mag_aim_reload
    _keyframe_loc_rot(magazine, 1, base_mag_loc, (0.0, 0.0, 0.0))
    _keyframe_loc_rot(magazine, 8, base_mag_loc, (0.0, 0.0, 0.0))
    _keyframe_loc_rot(magazine, 16, (base_mag_loc[0] + 0.020, 0.0, base_mag_loc[2] - 0.045), (0.0, -22.0, 0.0))
    _keyframe_loc_rot(magazine, 26, (base_mag_loc[0] + 0.048, 0.0, base_mag_loc[2] - 0.28), (0.0, -28.0, 0.0))
    _keyframe_loc_rot(magazine, 35, (base_mag_loc[0] + 0.020, 0.0, base_mag_loc[2] - 0.045), (0.0, -20.0, 0.0))
    _keyframe_loc_rot(magazine, 42, base_mag_loc, (0.0, 0.0, 0.0))
    _keyframe_loc_rot(magazine, 66, base_mag_loc, (0.0, 0.0, 0.0))
    push_action_to_nla(magazine, act_mag_aim_reload, "aim_reload", start_frame=1)

    act_bolt_aim_reload = bpy.data.actions.new("aim_reload_bolt")
    bolt_carrier.animation_data.action = act_bolt_aim_reload
    _keyframe_loc_rot(bolt_carrier, 1, base_bolt_loc)
    _keyframe_loc_rot(bolt_carrier, 45, base_bolt_loc)
    _keyframe_loc_rot(bolt_carrier, 52, (base_bolt_loc[0] - 0.088, base_bolt_loc[1], base_bolt_loc[2]))
    _keyframe_loc_rot(bolt_carrier, 58, base_bolt_loc)
    _keyframe_loc_rot(bolt_carrier, 66, base_bolt_loc)
    push_action_to_nla(bolt_carrier, act_bolt_aim_reload, "aim_reload", start_frame=1)


def build_classic_rifle(output_glb="godot/assets/models/classic_assault_rifle.glb"):
    """Construye el rifle clásico completo, sus animaciones y lo exporta a GLB."""
    clear_scene()
    col = get_or_create_collection("ClassicAssaultRifle")
    root, bolt_carrier, magazine = _build_rifle_geometry(col)
    _create_animations(root, bolt_carrier, magazine)
    bpy.context.scene.frame_start = 1
    bpy.context.scene.frame_end = 70
    bpy.context.scene.render.fps = 30

    if output_glb:
        export_glb(output_glb, export_animations=True)
    return root


if __name__ == "__main__":
    out_path = sys.argv[-1] if len(sys.argv) > 1 and sys.argv[-1].endswith(".glb") else "godot/assets/models/classic_assault_rifle.glb"
    build_classic_rifle(out_path)
