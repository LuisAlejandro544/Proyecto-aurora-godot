"""
Generador procedural en Blender (sin interfaz) del efecto 3D de explosión de granada.
Construye un núcleo de fuego incandescente, lóbulos de humo/llama, anillo expansivo
y esquirlas proyectadas con animación NLA `explode_burst` para instanciar en Godot.
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
    create_box,
    create_empty,
    export_glb,
    get_or_create_collection,
    link_to_collection,
    make_pbr_material,
    push_action_to_nla,
    set_smooth_shading,
)


def _create_sphere(name, radius, loc, mat, parent, collection):
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=2, radius=radius)
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    obj.location = loc
    bpy.context.scene.collection.objects.link(obj)
    link_to_collection(obj, collection)
    assign_material(obj, mat)
    set_smooth_shading(obj, smooth=True)
    obj.parent = parent
    return obj


def _create_shockwave_ring(name, mat, parent, collection):
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()

    segs = 24
    r_in = 0.72
    r_out = 1.0
    ring_in = [bm.verts.new((r_in * math.cos(2 * math.pi * i / segs), r_in * math.sin(2 * math.pi * i / segs), 0.02)) for i in range(segs)]
    ring_out = [bm.verts.new((r_out * math.cos(2 * math.pi * i / segs), r_out * math.sin(2 * math.pi * i / segs), 0.02)) for i in range(segs)]
    for i in range(segs):
        j = (i + 1) % segs
        bm.faces.new((ring_in[i], ring_out[i], ring_out[j], ring_in[j]))

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    link_to_collection(obj, collection)
    assign_material(obj, mat)
    obj.parent = parent
    return obj


def build_grenade_explosion_fx(output_glb="godot/assets/models/grenade_explosion_fx.glb"):
    """Construye la explosión 3D animada (`explode_burst`) y la exporta a GLB."""
    clear_scene()
    col = get_or_create_collection("GrenadeExplosionFX")

    mat_core = make_pbr_material(
        "Exp_Core",
        base_color=(1.0, 0.92, 0.62, 1.0),
        emission_color=(1.0, 0.85, 0.38, 1.0),
        emission_strength=14.0,
    )
    mat_fire = make_pbr_material(
        "Exp_Fire",
        base_color=(1.0, 0.36, 0.06, 0.92),
        emission_color=(1.0, 0.34, 0.04, 1.0),
        emission_strength=7.5,
        alpha=0.92,
    )
    mat_smoke = make_pbr_material(
        "Exp_Smoke",
        base_color=(0.24, 0.22, 0.20, 0.78),
        roughness=0.92,
        alpha=0.78,
    )
    mat_ring = make_pbr_material(
        "Exp_ShockRing",
        base_color=(1.0, 0.68, 0.28, 0.65),
        emission_color=(1.0, 0.58, 0.16, 1.0),
        emission_strength=4.5,
        alpha=0.65,
    )

    root = create_empty("ExplosionRoot", location=(0.0, 0.0, 0.0), collection=col)
    fire_cluster = create_empty("FireCluster", location=(0.0, 0.0, 0.0), parent=root, collection=col)

    _create_sphere("CoreFlash", 0.55, (0.0, 0.0, 0.35), mat_core, fire_cluster, col)
    lobe_offsets = [
        (0.45, 0.20, 0.55, 0.42),
        (-0.42, -0.25, 0.50, 0.40),
        (0.15, -0.45, 0.62, 0.38),
        (-0.20, 0.48, 0.48, 0.40),
        (0.0, 0.0, 0.95, 0.46),
    ]
    for idx, (lx, ly, lz, rad) in enumerate(lobe_offsets):
        _create_sphere(f"FireLobe_{idx}", rad, (lx, ly, lz), mat_fire, fire_cluster, col)
        _create_sphere(f"SmokeLobe_{idx}", rad * 1.18, (lx * 1.35, ly * 1.35, lz * 1.25), mat_smoke, fire_cluster, col)

    # Esquirlas radiales
    for i in range(8):
        ang = (2.0 * math.pi * i) / 8.0
        shard = create_box(
            f"BlastShard_{i}",
            size=(0.18, 0.05, 0.05),
            location=(math.cos(ang) * 0.75, math.sin(ang) * 0.75, 0.35 + (i % 3) * 0.22),
            rotation=(0.2 * i, -0.35, ang),
            material=mat_core,
            collection=col,
        )
        shard.parent = fire_cluster

    ring = _create_shockwave_ring("ShockwaveRing", mat_ring, root, col)

    # Animación de explosión (`explode_burst`, 28 frames a 60 fps = ~0.46s)
    act_fire = bpy.data.actions.new("explode_burst")
    fire_cluster.animation_data_create()
    fire_cluster.animation_data.action = act_fire
    for frame, sc, rz_deg in [
        (1, 0.18, 0.0),
        (5, 1.45, 18.0),
        (14, 2.35, 36.0),
        (28, 0.04, 58.0),
    ]:
        fire_cluster.scale = (sc, sc, sc)
        fire_cluster.rotation_euler = Euler((0.0, 0.0, math.radians(rz_deg)), "XYZ")
        fire_cluster.keyframe_insert(data_path="scale", frame=frame)
        fire_cluster.keyframe_insert(data_path="rotation_euler", frame=frame)
    push_action_to_nla(fire_cluster, act_fire, "explode_burst", start_frame=1)

    act_ring = bpy.data.actions.new("explode_burst_ring")
    ring.animation_data_create()
    ring.animation_data.action = act_ring
    for frame, sc in [
        (1, 0.25),
        (8, 2.8),
        (20, 4.8),
        (28, 0.02),
    ]:
        ring.scale = (sc, sc, 1.0)
        ring.keyframe_insert(data_path="scale", frame=frame)
    push_action_to_nla(ring, act_ring, "explode_burst", start_frame=1)

    bpy.context.scene.frame_start = 1
    bpy.context.scene.frame_end = 28
    bpy.context.scene.render.fps = 60

    if output_glb:
        export_glb(output_glb, export_animations=True)
    return root


if __name__ == "__main__":
    out_path = sys.argv[-1] if len(sys.argv) > 1 and sys.argv[-1].endswith(".glb") else "godot/assets/models/grenade_explosion_fx.glb"
    build_grenade_explosion_fx(out_path)
