"""
Generador procedural en Blender (sin interfaz) de la granada de fragmentación táctica.
Modela cuerpo segmentado verde oliva, collar superior de espoleta, palanca de seguridad
y anilla metálica, junto a animaciones NLA (`grenade_spin` y `grenade_arm`).
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
    set_smooth_shading,
)


def _create_segmented_grenade_body(name, mat_olive, parent, collection):
    """Construye el cuerpo ovoide acanalado de la granada de fragmentación."""
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()

    bmesh.ops.create_uvsphere(bm, u_segments=16, v_segments=12, radius=0.056)
    for v in bm.verts:
        # Alarga en eje Z vertical y marca sutiles surcos de fragmentación.
        z_norm = v.co.z / 0.056
        ang = math.atan2(v.co.y, v.co.x)
        groove = 1.0 - 0.08 * abs(math.sin(ang * 4.0)) - 0.06 * abs(math.sin(z_norm * math.pi * 2.5))
        v.co.x *= groove
        v.co.y *= groove
        v.co.z *= 1.32

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    link_to_collection(obj, collection)
    assign_material(obj, mat_olive)
    set_smooth_shading(obj, smooth=False)
    obj.parent = parent
    return obj


def _create_pull_ring(name, mat_steel, parent, collection):
    """Crea la anilla metálica de seguridad lateral."""
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()

    major_r = 0.022
    minor_r = 0.0028
    major_seg = 16
    minor_seg = 8
    rings = []
    for i in range(major_seg):
        theta = (2.0 * math.pi * i) / major_seg
        cx = major_r * math.cos(theta)
        cz = major_r * math.sin(theta)
        ring = []
        for j in range(minor_seg):
            phi = (2.0 * math.pi * j) / minor_seg
            r = major_r + minor_r * math.cos(phi)
            vx = r * math.cos(theta)
            vy = minor_r * math.sin(phi)
            vz = r * math.sin(theta)
            ring.append(bm.verts.new((vx, vy, vz)))
        rings.append(ring)

    for i in range(major_seg):
        inext = (i + 1) % major_seg
        for j in range(minor_seg):
            jnext = (j + 1) % minor_seg
            bm.faces.new((rings[i][j], rings[inext][j], rings[inext][jnext], rings[i][jnext]))

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()

    ring_obj = bpy.data.objects.new(name, mesh)
    ring_obj.location = (0.026, -0.022, 0.088)
    ring_obj.rotation_euler = Euler((math.radians(15.0), 0.0, math.radians(-25.0)), "XYZ")
    bpy.context.scene.collection.objects.link(ring_obj)
    link_to_collection(ring_obj, collection)
    assign_material(ring_obj, mat_steel)
    set_smooth_shading(ring_obj, smooth=True)
    ring_obj.parent = parent
    return ring_obj


def build_frag_grenade(output_glb="godot/assets/models/frag_grenade.glb"):
    """Construye el modelo completo de granada táctica y lo exporta a GLB."""
    clear_scene()
    col = get_or_create_collection("FragGrenade")

    mat_olive = make_pbr_material(
        "Grenade_Olive",
        base_color=(0.16, 0.24, 0.13, 1.0),
        metallic=0.25,
        roughness=0.58,
    )
    mat_band = make_pbr_material(
        "Grenade_YellowBand",
        base_color=(0.88, 0.64, 0.12, 1.0),
        metallic=0.15,
        roughness=0.42,
    )
    mat_steel = make_pbr_material(
        "Grenade_Steel",
        base_color=(0.45, 0.47, 0.50, 1.0),
        metallic=0.90,
        roughness=0.28,
    )
    mat_lever = make_pbr_material(
        "Grenade_Lever",
        base_color=(0.24, 0.26, 0.24, 1.0),
        metallic=0.78,
        roughness=0.36,
    )

    root = create_empty("FragGrenadeRoot", location=(0.0, 0.0, 0.0), collection=col)
    spin_pivot = create_empty("GrenadeSpinPivot", location=(0.0, 0.0, 0.0), parent=root, collection=col)

    _create_segmented_grenade_body("GrenadeBody", mat_olive, spin_pivot, col)

    collar = create_cylinder(
        "FuseCollar",
        radius=0.022,
        depth=0.032,
        segments=14,
        location=(0.0, 0.0, 0.078),
        material=mat_steel,
        collection=col,
    )
    collar.parent = spin_pivot

    band = create_cylinder(
        "HazardBand",
        radius=0.023,
        depth=0.008,
        segments=14,
        location=(0.0, 0.0, 0.068),
        material=mat_band,
        collection=col,
    )
    band.parent = spin_pivot

    fuse_top = create_box(
        "FuseHead",
        size=(0.030, 0.024, 0.024),
        location=(0.008, 0.0, 0.098),
        material=mat_steel,
        bevel_width=0.002,
        collection=col,
    )
    fuse_top.parent = spin_pivot

    spoon_top = create_box(
        "SafetySpoonTop",
        size=(0.038, 0.018, 0.006),
        location=(0.024, 0.0, 0.104),
        rotation=(0.0, math.radians(12.0), 0.0),
        material=mat_lever,
        bevel_width=0.001,
        collection=col,
    )
    spoon_top.parent = spin_pivot

    spoon_side = create_box(
        "SafetySpoonSide",
        size=(0.006, 0.018, 0.088),
        location=(0.055, 0.0, 0.056),
        rotation=(0.0, math.radians(-14.0), 0.0),
        material=mat_lever,
        bevel_width=0.001,
        collection=col,
    )
    spoon_side.parent = spin_pivot

    _create_pull_ring("SafetyRing", mat_steel, spin_pivot, col)

    # Animación de giro en vuelo (`grenade_spin`)
    act_spin = bpy.data.actions.new("grenade_spin")
    spin_pivot.animation_data_create()
    spin_pivot.animation_data.action = act_spin
    for frame, deg_x, deg_y, deg_z in [
        (1, 0.0, 0.0, 0.0),
        (11, 180.0, 90.0, 45.0),
        (21, 360.0, 180.0, 90.0),
    ]:
        spin_pivot.rotation_euler = Euler(
            (math.radians(deg_x), math.radians(deg_y), math.radians(deg_z)),
            "XYZ",
        )
        spin_pivot.keyframe_insert(data_path="rotation_euler", frame=frame)
    push_action_to_nla(spin_pivot, act_spin, "grenade_spin", start_frame=1)

    bpy.context.scene.frame_start = 1
    bpy.context.scene.frame_end = 21
    bpy.context.scene.render.fps = 30

    if output_glb:
        export_glb(output_glb, export_animations=True)
    return root


if __name__ == "__main__":
    out_path = sys.argv[-1] if len(sys.argv) > 1 and sys.argv[-1].endswith(".glb") else "godot/assets/models/frag_grenade.glb"
    build_frag_grenade(out_path)
