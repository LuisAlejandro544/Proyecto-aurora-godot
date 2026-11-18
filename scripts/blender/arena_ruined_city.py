"""
Generador procedural en Blender (sin interfaz) del primer escenario 3D:
una pequeña ciudad destruida (~38m x 38m) optimizada para combate Jugador vs IA
con abundante cobertura táctica, edificios en ruinas transitables, cráteres
con hollín, barreras de hormigón, sacos de arena detallados, cajas y vehículos calcinados.

Incluye sufijo '-col' en las mallas de colisión estática para que Godot 4.7
genere automáticamente los StaticBody3D al importar el GLB, además de los nodos
`SpawnPlayer` y `SpawnAI`.
"""

import math
import os
import random
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
)


def _create_materials():
    """Paleta PBR desgastada y atmosférica de ciudad destruida."""
    return {
        "asphalt": make_pbr_material("City_Asphalt", (0.11, 0.115, 0.122, 1.0), metallic=0.0, roughness=0.94),
        "road_line": make_pbr_material("City_RoadLine", (0.64, 0.54, 0.20, 1.0), metallic=0.0, roughness=0.84),
        "sidewalk": make_pbr_material("City_Sidewalk", (0.26, 0.26, 0.27, 1.0), metallic=0.0, roughness=0.90),
        "concrete_wall": make_pbr_material("City_ConcreteWall", (0.31, 0.30, 0.28, 1.0), metallic=0.0, roughness=0.88),
        "concrete_dark": make_pbr_material("City_ConcreteDark", (0.20, 0.195, 0.19, 1.0), metallic=0.0, roughness=0.92),
        "brick_ruin": make_pbr_material("City_BrickRuin", (0.32, 0.13, 0.085, 1.0), metallic=0.0, roughness=0.88),
        "rubble": make_pbr_material("City_Rubble", (0.22, 0.21, 0.19, 1.0), metallic=0.0, roughness=0.94),
        "soot": make_pbr_material("City_Soot", (0.05, 0.048, 0.045, 1.0), metallic=0.0, roughness=0.96),
        "rebar": make_pbr_material("City_RebarRust", (0.26, 0.11, 0.06, 1.0), metallic=0.72, roughness=0.68),
        "barrier": make_pbr_material("City_JerseyBarrier", (0.42, 0.42, 0.40, 1.0), metallic=0.0, roughness=0.85),
        "hazard": make_pbr_material("City_HazardYellow", (0.82, 0.52, 0.08, 1.0), metallic=0.05, roughness=0.58),
        "sandbag": make_pbr_material("City_Sandbag", (0.45, 0.38, 0.25, 1.0), metallic=0.0, roughness=0.95),
        "crate_wood": make_pbr_material("City_CrateWood", (0.32, 0.20, 0.09, 1.0), metallic=0.0, roughness=0.80),
        "crate_metal": make_pbr_material("City_CrateMetal", (0.16, 0.22, 0.17, 1.0), metallic=0.68, roughness=0.48),
        "wreck_body": make_pbr_material("City_WreckBody", (0.16, 0.12, 0.10, 1.0), metallic=0.52, roughness=0.76),
        "wreck_tire": make_pbr_material("City_WreckTire", (0.045, 0.045, 0.045, 1.0), metallic=0.0, roughness=0.94),
        "beacon_player": make_pbr_material(
            "City_BeaconPlayer",
            (0.15, 0.72, 1.0, 1.0),
            roughness=0.25,
            emission_color=(0.15, 0.72, 1.0, 1.0),
            emission_strength=3.0,
        ),
        "beacon_ai": make_pbr_material(
            "City_BeaconAI",
            (1.0, 0.28, 0.12, 1.0),
            roughness=0.25,
            emission_color=(1.0, 0.28, 0.12, 1.0),
            emission_strength=3.0,
        ),
    }


def _build_ground_and_streets(mats, parent, col):
    """Crea la base de asfalto, aceras perimetrales, líneas viales y muros límite dañados."""
    ground = create_box(
        "GroundAsphalt-col",
        size=(40.0, 40.0, 0.4),
        location=(0.0, 0.0, -0.2),
        material=mats["asphalt"],
        collection=col,
    )
    ground.parent = parent

    perimeter_specs = [
        ("Perimeter_N-col", (40.8, 0.8, 3.0), (0.0, 20.0, 1.5)),
        ("Perimeter_S-col", (40.8, 0.8, 3.0), (0.0, -20.0, 1.5)),
        ("Perimeter_E-col", (0.8, 40.0, 3.0), (20.0, 0.0, 1.5)),
        ("Perimeter_W-col", (0.8, 40.0, 3.0), (-20.0, 0.0, 1.5)),
    ]
    for name, sz, loc in perimeter_specs:
        wall = create_box(name, size=sz, location=loc, material=mats["concrete_dark"], bevel_width=0.03, collection=col)
        wall.parent = parent

    sidewalks = [
        ("Sidewalk_NE-col", (14.5, 14.5, 0.18), (12.2, 12.2, 0.09)),
        ("Sidewalk_NW-col", (14.5, 14.5, 0.18), (-12.2, 12.2, 0.09)),
        ("Sidewalk_SE-col", (14.5, 14.5, 0.18), (12.2, -12.2, 0.09)),
        ("Sidewalk_SW-col", (14.5, 14.5, 0.18), (-12.2, -12.2, 0.09)),
    ]
    for name, sz, loc in sidewalks:
        sw = create_box(name, size=sz, location=loc, material=mats["sidewalk"], bevel_width=0.02, collection=col)
        sw.parent = parent

    for idx, pos in enumerate((-15.0, -10.0, -5.0, 5.0, 10.0, 15.0)):
        lx = create_box(f"RoadLine_X_{idx}", size=(2.2, 0.22, 0.02), location=(pos, 0.0, 0.01), material=mats["road_line"], collection=col)
        lx.parent = parent
        ly = create_box(f"RoadLine_Y_{idx}", size=(0.22, 2.2, 0.02), location=(0.0, pos, 0.01), material=mats["road_line"], collection=col)
        ly.parent = parent


def _build_ruined_building(name, center_xy, yaw_deg, mats, parent, col, has_ramp=False, seed=10):
    """
    Construye un edificio destruido sin z-fighting en esquinas, con topes dentados
    por demolición, vigas rotas, ladrillos caídos y ventanas transitables.
    """
    rng = random.Random(seed)
    cx, cy = center_xy
    b_root = create_empty(name, location=(cx, cy, 0.18), rotation=(0.0, 0.0, math.radians(yaw_deg)), parent=parent, collection=col)

    floor1 = create_box(f"{name}_Floor1-col", size=(8.4, 8.4, 0.12), location=(0.0, 0.0, 0.06), material=mats["concrete_dark"], collection=col)
    floor1.parent = b_root

    # Muros acortados para no solaparse con los pilares de esquina (evita z-fighting)
    wall_parts = [
        ("WallN_L-col", (2.25, 0.34, 3.5), (-2.55, 3.9, 1.75), "concrete_wall"),
        ("WallN_R-col", (2.25, 0.34, 3.0), (2.55, 3.9, 1.50), "concrete_wall"),
        ("WallN_Sill-col", (2.85, 0.32, 1.05), (0.0, 3.9, 0.525), "brick_ruin"),
        ("WallS_L-col", (2.35, 0.34, 3.7), (-2.50, -3.9, 1.85), "concrete_wall"),
        ("WallS_R-col", (2.35, 0.34, 2.2), (2.50, -3.9, 1.10), "brick_ruin"),
        ("WallE_B-col", (0.34, 2.9, 3.3), (3.9, -2.20, 1.65), "concrete_wall"),
        ("WallE_Sill-col", (0.32, 4.3, 1.10), (3.9, 1.45, 0.55), "brick_ruin"),
        ("WallW_A-col", (0.34, 3.2, 4.0), (-3.9, 2.05, 2.00), "concrete_wall"),
        ("WallW_B-col", (0.34, 2.1, 2.6), (-3.9, -2.60, 1.30), "brick_ruin"),
    ]
    for wname, sz, loc, mkey in wall_parts:
        w = create_box(f"{name}_{wname}", size=sz, location=loc, material=mats[mkey], bevel_width=0.025, collection=col)
        w.parent = b_root

    # Fragmentos dentados de mampostería sobre los muros rotos (rompe la silueta recta)
    jagged_specs = [
        ((-2.2, 3.9, 3.65), (0.9, 0.30, 0.45), "brick_ruin"),
        ((2.2, 3.9, 3.15), (0.8, 0.30, 0.40), "concrete_wall"),
        ((-3.9, 1.8, 4.15), (0.30, 1.1, 0.45), "brick_ruin"),
        ((3.9, -2.0, 3.45), (0.30, 0.9, 0.42), "concrete_wall"),
    ]
    for idx, (jloc, jsz, jm) in enumerate(jagged_specs):
        j = create_box(
            f"{name}_Jagged_{idx}",
            size=jsz,
            location=jloc,
            rotation=(rng.uniform(-0.08, 0.08), rng.uniform(-0.08, 0.08), rng.uniform(-0.05, 0.05)),
            material=mats[jm],
            bevel_width=0.02,
            collection=col,
        )
        j.parent = b_root

    # Losa parcial de 2º piso y pretil con cobertura
    slab2 = create_box(f"{name}_Floor2-col", size=(4.5, 4.5, 0.22), location=(-1.65, 1.65, 3.1), material=mats["concrete_dark"], bevel_width=0.03, collection=col)
    slab2.parent = b_root

    parapet = create_box(f"{name}_Parapet2-col", size=(4.1, 0.24, 1.05), location=(-1.65, -0.45, 3.65), material=mats["brick_ruin"], bevel_width=0.02, collection=col)
    parapet.parent = b_root

    # Pilares de esquina ligeramente más anchos (0.64) para evitar solapamiento exacto con muros
    for idx, (px, py, ph) in enumerate([(-3.9, 3.9, 4.7), (3.9, 3.9, 3.6), (-3.9, -3.9, 4.0), (3.9, -3.9, 2.7)]):
        pillar = create_box(f"{name}_Pillar_{idx}-col", size=(0.64, 0.64, ph), location=(px, py, ph * 0.5), material=mats["concrete_wall"], bevel_width=0.025, collection=col)
        pillar.parent = b_root
        for r_i in range(2):
            rebar = create_cylinder(
                f"{name}_Rebar_{idx}_{r_i}",
                radius=0.022,
                depth=0.75,
                segments=6,
                location=(px + (0.12 if r_i == 0 else -0.10), py - 0.08, ph + 0.28),
                rotation=(0.18 * (r_i + 1), -0.14, 0.0),
                material=mats["rebar"],
                collection=col,
            )
            rebar.parent = b_root

    # Escombros caídos dentro del edificio
    for d_i in range(4):
        dx = rng.uniform(-2.6, 2.6)
        dy = rng.uniform(-2.6, 2.6)
        debris = create_box(
            f"{name}_Debris_{d_i}",
            size=(rng.uniform(0.35, 0.7), rng.uniform(0.25, 0.5), rng.uniform(0.16, 0.28)),
            location=(dx, dy, 0.20),
            rotation=(rng.uniform(-0.2, 0.2), rng.uniform(-0.2, 0.2), rng.uniform(0, 3.14)),
            material=mats["rubble"],
            bevel_width=0.02,
            collection=col,
        )
        debris.parent = b_root

    if has_ramp:
        ramp = create_box(
            f"{name}_Ramp-col",
            size=(2.0, 5.2, 0.24),
            location=(1.65, -0.45, 1.55),
            rotation=(math.radians(33.0), 0.0, 0.0),
            material=mats["concrete_wall"],
            bevel_width=0.03,
            collection=col,
        )
        ramp.parent = b_root


def _build_jersey_barrier(name, location, yaw_deg, mats, parent, col, hazard=False):
    """Construye una barrera vial tipo Jersey de hormigón (cobertura media ~1.1m)."""
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()

    profile = [
        (-0.32, 0.0),
        (0.32, 0.0),
        (0.28, 0.22),
        (0.14, 0.45),
        (0.10, 1.08),
        (-0.10, 1.08),
        (-0.14, 0.45),
        (-0.28, 0.22),
    ]
    hx = 1.2
    v_l = [bm.verts.new((-hx, y, z)) for (y, z) in profile]
    v_r = [bm.verts.new((hx, y, z)) for (y, z) in profile]
    bm.faces.new(v_l[::-1])
    bm.faces.new(v_r)
    n = len(profile)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((v_l[i], v_l[j], v_r[j], v_r[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    obj.location = location
    obj.rotation_euler = Euler((0.0, 0.0, math.radians(yaw_deg)), "XYZ")
    bpy.context.scene.collection.objects.link(obj)
    link_to_collection(obj, col)
    assign_material(obj, mats["barrier"])
    add_bevel(obj, width=0.02, segments=2)
    obj.parent = parent

    if hazard:
        stripe = create_box(
            f"{name}_Stripe",
            size=(2.2, 0.22, 0.14),
            location=(0.0, 0.0, 0.96),
            material=mats["hazard"],
            collection=col,
        )
        stripe.parent = obj


def _build_wrecked_vehicle(name, location, yaw_deg, mats, parent, col):
    """Construye la silueta de un vehículo calcinado con techo hundido."""
    v_root = create_empty(name, location=location, rotation=(0.0, 0.0, math.radians(yaw_deg)), parent=parent, collection=col)

    chassis = create_box(f"{name}_Chassis-col", size=(4.2, 1.75, 0.72), location=(0.0, 0.0, 0.52), material=mats["wreck_body"], bevel_width=0.06, collection=col)
    chassis.parent = v_root

    cabin = create_box(
        f"{name}_Cabin-col",
        size=(1.95, 1.54, 0.58),
        location=(-0.15, 0.0, 1.12),
        rotation=(0.03, -0.04, 0.0),
        material=mats["wreck_body"],
        bevel_width=0.07,
        collection=col,
    )
    cabin.parent = v_root

    for idx, (wx, wy) in enumerate([(1.32, 0.86), (1.32, -0.86), (-1.32, 0.86), (-1.32, -0.86)]):
        wheel = create_cylinder(
            f"{name}_Wheel_{idx}",
            radius=0.34,
            depth=0.22,
            segments=12,
            location=(wx, wy, 0.34),
            rotation=(math.radians(90.0), 0.0, 0.0),
            material=mats["wreck_tire"],
            collection=col,
        )
        wheel.parent = v_root


def _build_sandbag_bunker(name, location, yaw_deg, mats, parent, col):
    """Construye una trinchera de sacos de arena visibles individualmente (colisionador interior oculto)."""
    sb_root = create_empty(name, location=location, rotation=(0.0, 0.0, math.radians(yaw_deg)), parent=parent, collection=col)
    # Colisionador más pequeño en el núcleo para que los sacos exteriores resalten visualmente
    collider = create_box(f"{name}-col", size=(2.55, 0.36, 0.92), location=(0.0, 0.0, 0.46), material=mats["sandbag"], collection=col)
    collider.parent = sb_root

    for row in range(3):
        for i in range(4):
            offset_x = -0.96 + i * 0.64 + (0.14 if row % 2 == 1 else -0.06)
            bag = create_box(
                f"{name}_Bag_{row}_{i}",
                size=(0.60, 0.44, 0.29),
                location=(offset_x, 0.0, 0.16 + row * 0.31),
                rotation=(0.0, 0.0, 0.04 if (i + row) % 2 == 0 else -0.04),
                material=mats["sandbag"],
                bevel_width=0.07,
                collection=col,
            )
            bag.parent = sb_root


def _build_crater_and_rubble(name, location, radius, mats, parent, col, seed=42):
    """Crea un cráter de explosión con mancha oscura de hollín y anillo de escombros."""
    rng = random.Random(seed)
    c_root = create_empty(name, location=location, parent=parent, collection=col)

    soot_disc = create_cylinder(
        f"{name}_Soot",
        radius=radius * 0.95,
        depth=0.025,
        segments=16,
        location=(0.0, 0.0, 0.012),
        material=mats["soot"],
        collection=col,
    )
    soot_disc.parent = c_root

    pieces = 10
    for i in range(pieces):
        ang = (2.0 * math.pi * i) / pieces
        r = radius * rng.uniform(0.85, 1.12)
        px = r * math.cos(ang)
        py = r * math.sin(ang)
        sx = rng.uniform(0.6, 1.25)
        sy = rng.uniform(0.5, 1.0)
        sz = rng.uniform(0.28, 0.62)
        block = create_box(
            f"{name}_Chunk_{i}-col",
            size=(sx, sy, sz),
            location=(px, py, sz * 0.45),
            rotation=(rng.uniform(-0.18, 0.18), rng.uniform(-0.18, 0.18), ang),
            material=mats["rubble"],
            bevel_width=0.04,
            collection=col,
        )
        block.parent = c_root


def _build_ruined_streetlights(mats, parent, col):
    """Añade postes de luz torcidos o quebrados en las esquinas de la intersección."""
    specs = [
        ("Lamp_NE-col", (4.8, 4.8, 0.18), 14.0, -10.0),
        ("Lamp_SW-col", (-4.8, -4.8, 0.18), -12.0, 8.0),
    ]
    for name, loc, tilt_x, tilt_y in specs:
        pole = create_cylinder(
            name,
            radius=0.08,
            depth=4.2,
            segments=8,
            location=(loc[0], loc[1], loc[2] + 2.0),
            rotation=(math.radians(tilt_x), math.radians(tilt_y), 0.0),
            material=mats["rebar"],
            collection=col,
        )
        pole.parent = parent


def build_ruined_city(output_glb="godot/assets/models/ruined_city_arena.glb"):
    """Genera el escenario completo de ciudad destruida con cobertura densa y lo exporta a GLB."""
    clear_scene()
    col = get_or_create_collection("RuinedCityArena")
    mats = _create_materials()
    root = create_empty("RuinedCityRoot", location=(0.0, 0.0, 0.0), collection=col)

    _build_ground_and_streets(mats, root, col)

    _build_ruined_building("Building_NE", (12.2, 12.2), 180.0, mats, root, col, has_ramp=True, seed=11)
    _build_ruined_building("Building_SW", (-12.2, -12.2), 0.0, mats, root, col, has_ramp=True, seed=22)
    _build_ruined_building("Building_NW", (-12.2, 12.2), -90.0, mats, root, col, has_ramp=False, seed=33)
    _build_ruined_building("Building_SE", (12.2, -12.2), 90.0, mats, root, col, has_ramp=False, seed=44)

    barriers = [
        ("Barrier_Center_1-col", (-2.6, 2.2, 0.0), 28.0, True),
        ("Barrier_Center_2-col", (2.8, -2.0, 0.0), -32.0, True),
        ("Barrier_North_1-col", (-1.5, 8.8, 0.0), 8.0, False),
        ("Barrier_North_2-col", (1.6, 9.6, 0.0), -12.0, True),
        ("Barrier_South_1-col", (1.5, -8.8, 0.0), 12.0, False),
        ("Barrier_South_2-col", (-1.6, -9.6, 0.0), -8.0, True),
        ("Barrier_East_1-col", (9.0, 1.6, 0.0), 95.0, False),
        ("Barrier_West_1-col", (-9.0, -1.6, 0.0), 85.0, False),
    ]
    for bname, bloc, byaw, bhaz in barriers:
        _build_jersey_barrier(bname, bloc, byaw, mats, root, col, hazard=bhaz)

    # Vehículos calcinados sin solaparse con sacos de arena
    _build_wrecked_vehicle("Wreck_Car_CenterW", (-4.8, -0.8, 0.0), 18.0, mats, root, col)
    _build_wrecked_vehicle("Wreck_Car_CenterE", (5.2, 1.4, 0.0), -155.0, mats, root, col)
    _build_wrecked_vehicle("Wreck_Car_North", (-3.2, 13.4, 0.0), 74.0, mats, root, col)

    _build_sandbag_bunker("Sandbags_PlayerSide", (0.0, -14.2, 0.0), 0.0, mats, root, col)
    _build_sandbag_bunker("Sandbags_AISide", (0.8, 14.2, 0.0), 180.0, mats, root, col)
    _build_sandbag_bunker("Sandbags_FlankE", (15.5, 0.0, 0.0), 90.0, mats, root, col)
    _build_sandbag_bunker("Sandbags_FlankW", (-15.5, 0.0, 0.0), 90.0, mats, root, col)

    crates = [
        ("CrateStack_1-col", (3.8, 5.5, 0.55), (1.1, 1.1, 1.1), 15.0, "crate_metal"),
        ("CrateStack_2-col", (4.5, 6.2, 0.45), (0.9, 0.9, 0.9), -20.0, "crate_wood"),
        ("CrateStack_3-col", (-4.2, -5.6, 0.55), (1.1, 1.1, 1.1), -12.0, "crate_metal"),
        ("CrateStack_4-col", (-5.0, -6.1, 0.45), (0.9, 0.9, 0.9), 30.0, "crate_wood"),
        ("CrateStack_5-col", (-6.2, 4.8, 0.55), (1.2, 1.0, 1.1), 42.0, "crate_wood"),
        ("CrateStack_6-col", (6.2, -4.8, 0.55), (1.2, 1.0, 1.1), -38.0, "crate_metal"),
    ]
    for cname, cloc, csize, cyaw, cmat in crates:
        c = create_box(cname, size=csize, location=cloc, rotation=(0.0, 0.0, math.radians(cyaw)), material=mats[cmat], bevel_width=0.03, collection=col)
        c.parent = root

    _build_crater_and_rubble("Crater_Center", (0.0, 0.0, 0.0), radius=2.1, mats=mats, parent=root, col=col, seed=7)
    _build_crater_and_rubble("Crater_MidEast", (6.5, -1.5, 0.0), radius=1.6, mats=mats, parent=root, col=col, seed=19)
    _build_crater_and_rubble("Crater_MidWest", (-6.8, 2.2, 0.0), radius=1.6, mats=mats, parent=root, col=col, seed=31)
    _build_ruined_streetlights(mats, root, col)

    beacon_p = create_cylinder("Beacon_Player", radius=0.22, depth=0.10, segments=12, location=(0.0, -16.8, 0.05), material=mats["beacon_player"], collection=col)
    beacon_p.parent = root
    beacon_ai = create_cylinder("Beacon_AI", radius=0.22, depth=0.10, segments=12, location=(0.0, 16.8, 0.05), material=mats["beacon_ai"], collection=col)
    beacon_ai.parent = root

    create_empty("SpawnPlayer", location=(0.0, -16.8, 0.1), rotation=(0.0, 0.0, math.radians(90.0)), parent=root, collection=col)
    create_empty("SpawnAI", location=(0.0, 16.8, 0.1), rotation=(0.0, 0.0, math.radians(-90.0)), parent=root, collection=col)

    if output_glb:
        export_glb(output_glb, export_animations=False)
    return root


if __name__ == "__main__":
    out_path = sys.argv[-1] if len(sys.argv) > 1 and sys.argv[-1].endswith(".glb") else "godot/assets/models/ruined_city_arena.glb"
    build_ruined_city(out_path)
