"""
Generador procedural en Blender (sin interfaz) del efecto de fogonazo (Muzzle Flash).
Crea una estrella volumétrica multicapa de 3 aletas cruzadas y un núcleo incandescente
con animación rápida ('flash_burst') para acoplar al nodo `MuzzlePoint` del arma en Godot.
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
    assign_material,
    clear_scene,
    create_empty,
    export_glb,
    get_or_create_collection,
    link_to_collection,
    make_pbr_material,
    push_action_to_nla,
    set_smooth_shading,
)


def _create_flash_cone_fin(name, length, radius, num_spikes, mat, parent, collection):
    """Crea un cono estrellado para simular llamas del disparo desde cualquier ángulo."""
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()

    center_tip = bm.verts.new((length, 0.0, 0.0))
    base_center = bm.verts.new((0.0, 0.0, 0.0))

    ring_verts = []
    total_pts = num_spikes * 2
    for i in range(total_pts):
        ang = (2.0 * math.pi * i) / total_pts
        r = radius if (i % 2 == 0) else radius * 0.28
        x = 0.055 if (i % 2 == 0) else 0.015
        v = bm.verts.new((x, r * math.cos(ang), r * math.sin(ang)))
        ring_verts.append(v)

    for i in range(total_pts):
        jn = (i + 1) % total_pts
        bm.faces.new((base_center, ring_verts[i], ring_verts[jn]))
        bm.faces.new((ring_verts[i], center_tip, ring_verts[jn]))

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    link_to_collection(obj, collection)
    assign_material(obj, mat)
    set_smooth_shading(obj, smooth=False)
    obj.parent = parent
    return obj


def _create_side_wings(name, mat, parent, collection):
    """Crea chispas laterales características de un freno de boca biselado."""
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()

    for sign in (-1.0, 1.0):
        v0 = bm.verts.new((0.01, 0.0, 0.0))
        v1 = bm.verts.new((0.055, sign * 0.115, 0.015))
        v2 = bm.verts.new((0.045, sign * 0.045, -0.020))
        v3 = bm.verts.new((0.025, 0.0, 0.020))
        bm.faces.new((v0, v1, v2))
        bm.faces.new((v0, v3, v1))
        bm.faces.new((v0, v2, v3))

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    link_to_collection(obj, collection)
    assign_material(obj, mat)
    obj.parent = parent
    return obj


def build_muzzle_flash(output_glb="godot/assets/models/muzzle_flash_fx.glb"):
    """Construye el efecto de fogonazo animado y lo exporta a GLB."""
    clear_scene()
    col = get_or_create_collection("MuzzleFlashFX")

    mat_core = make_pbr_material(
        "FX_Flash_Core",
        base_color=(1.0, 0.96, 0.72, 1.0),
        metallic=0.0,
        roughness=0.2,
        emission_color=(1.0, 0.92, 0.55, 1.0),
        emission_strength=14.0,
    )
    mat_outer = make_pbr_material(
        "FX_Flash_Outer",
        base_color=(1.0, 0.45, 0.06, 0.85),
        metallic=0.0,
        roughness=0.35,
        emission_color=(1.0, 0.42, 0.05, 1.0),
        emission_strength=8.5,
        alpha=0.85,
    )

    root = create_empty("MuzzleFlashRoot", location=(0.0, 0.0, 0.0), collection=col)

    _create_flash_cone_fin("FlashCore", length=0.145, radius=0.042, num_spikes=6, mat=mat_core, parent=root, collection=col)
    _create_flash_cone_fin("FlashOuter", length=0.235, radius=0.075, num_spikes=8, mat=mat_outer, parent=root, collection=col)
    _create_side_wings("FlashSideSparks", mat=mat_outer, parent=root, collection=col)

    # Animación de destello ultra rápida (frames 1..7)
    act = bpy.data.actions.new("flash_burst")
    root.animation_data_create()
    root.animation_data.action = act

    keyframes = [
        (1, (0.02, 0.02, 0.02), 0.0),
        (2, (1.15, 1.15, 1.15), 25.0),
        (4, (0.85, 0.95, 0.95), 70.0),
        (6, (0.35, 0.35, 0.35), 115.0),
        (7, (0.001, 0.001, 0.001), 130.0),
    ]
    for frame, scl, rot_x in keyframes:
        root.scale = scl
        root.rotation_euler = Euler((math.radians(rot_x), 0.0, 0.0), "XYZ")
        root.keyframe_insert(data_path="scale", frame=frame)
        root.keyframe_insert(data_path="rotation_euler", frame=frame)

    push_action_to_nla(root, act, "flash_burst", start_frame=1)
    bpy.context.scene.frame_start = 1
    bpy.context.scene.frame_end = 7
    bpy.context.scene.render.fps = 60

    if output_glb:
        export_glb(output_glb, export_animations=True)
    return root


if __name__ == "__main__":
    out_path = sys.argv[-1] if len(sys.argv) > 1 and sys.argv[-1].endswith(".glb") else "godot/assets/models/muzzle_flash_fx.glb"
    build_muzzle_flash(out_path)
