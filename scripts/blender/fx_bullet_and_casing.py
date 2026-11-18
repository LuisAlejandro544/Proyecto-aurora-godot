"""
Generador procedural en Blender (sin interfaz) de proyectiles 7.62 y casquillos
expulsados con animación de salida y caída al suelo.

Exporta dos archivos GLB independientes:
1. `bullet_projectile.glb` : bala ojival encamisada en cobre para disparos/trazadoras.
2. `shell_casing_eject.glb`: casquillo de latón vacío con animación ('casing_eject')
   donde sale expulsado hacia la derecha, gira en el aire, cae por gravedad y rebota en el suelo.
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


def _create_lathe_mesh(name, profile_rz, segments, mat, parent, collection):
    """Genera un sólido de revolución a lo largo del eje +X a partir de pares (x, radio)."""
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()

    rings = []
    for (x, r) in profile_rz:
        if r <= 1e-6:
            v = bm.verts.new((x, 0.0, 0.0))
            rings.append([v])
        else:
            ring = []
            for i in range(segments):
                ang = (2.0 * math.pi * i) / segments
                ring.append(bm.verts.new((x, r * math.cos(ang), r * math.sin(ang))))
            rings.append(ring)

    for idx in range(len(rings) - 1):
        r0 = rings[idx]
        r1 = rings[idx + 1]
        if len(r0) == 1 and len(r1) > 1:
            tip = r0[0]
            for i in range(segments):
                jn = (i + 1) % segments
                bm.faces.new((tip, r1[jn], r1[i]))
        elif len(r0) > 1 and len(r1) == 1:
            tip = r1[0]
            for i in range(segments):
                jn = (i + 1) % segments
                bm.faces.new((r0[i], r0[jn], tip))
        elif len(r0) > 1 and len(r1) > 1:
            for i in range(segments):
                jn = (i + 1) % segments
                bm.faces.new((r0[i], r0[jn], r1[jn], r1[i]))

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    link_to_collection(obj, collection)
    assign_material(obj, mat)
    set_smooth_shading(obj, smooth=True)
    if parent is not None:
        obj.parent = parent
    return obj


def build_bullet_projectile(output_glb="godot/assets/models/bullet_projectile.glb"):
    """Construye la bala ojival 7.62 encamisada en cobre con base trazadora."""
    clear_scene()
    col = get_or_create_collection("BulletProjectile")

    mat_copper = make_pbr_material(
        "Bullet_Copper",
        base_color=(0.82, 0.44, 0.18, 1.0),
        metallic=0.92,
        roughness=0.22,
    )
    mat_tracer = make_pbr_material(
        "Bullet_Tracer_Glow",
        base_color=(1.0, 0.55, 0.1, 1.0),
        metallic=0.0,
        roughness=0.3,
        emission_color=(1.0, 0.48, 0.08, 1.0),
        emission_strength=10.0,
    )

    root = create_empty("BulletRoot", location=(0.0, 0.0, 0.0), collection=col)

    # Perfil real aproximado de proyectil 7.62x39 (escala ligeramente aumentada para lectura móvil)
    bullet_profile = [
        (-0.015, 0.0),
        (-0.015, 0.0042),
        (-0.008, 0.0052),
        (0.006, 0.0052),
        (0.018, 0.0038),
        (0.028, 0.0014),
        (0.032, 0.0),
    ]
    _create_lathe_mesh("BulletJacket", bullet_profile, segments=14, mat=mat_copper, parent=root, collection=col)

    tracer_profile = [
        (-0.020, 0.0),
        (-0.018, 0.0036),
        (-0.014, 0.0036),
        (-0.014, 0.0),
    ]
    _create_lathe_mesh("BulletTracerBase", tracer_profile, segments=12, mat=mat_tracer, parent=root, collection=col)

    if output_glb:
        export_glb(output_glb, export_animations=False)
    return root


def build_shell_casing_eject(output_glb="godot/assets/models/shell_casing_eject.glb"):
    """
    Construye el casquillo vacío de latón 7.62x39 con su animación de expulsión y caída.
    El origen (0,0,0) representa el puerto de expulsión del rifle (~1.35 m sobre el suelo).
    """
    clear_scene()
    col = get_or_create_collection("ShellCasingEject")

    mat_brass = make_pbr_material(
        "Casing_Brass",
        base_color=(0.86, 0.68, 0.24, 1.0),
        metallic=0.94,
        roughness=0.26,
    )
    mat_primer = make_pbr_material(
        "Casing_Primer",
        base_color=(0.55, 0.56, 0.58, 1.0),
        metallic=0.90,
        roughness=0.32,
    )

    root = create_empty("CasingEjectRoot", location=(0.0, 0.0, 0.0), collection=col)

    # Perfil cónico tipo botella 7.62x39 (escala 1.5x para visibilidad nítida en móvil)
    casing_profile = [
        (-0.028, 0.0),
        (-0.028, 0.0085),
        (-0.026, 0.0085),
        (-0.025, 0.0072),
        (-0.023, 0.0085),
        (0.014, 0.0075),
        (0.020, 0.0062),
        (0.028, 0.0062),
        (0.028, 0.0052),
        (0.018, 0.0052),
        (0.018, 0.0),
    ]
    casing = _create_lathe_mesh("ShellCasing", casing_profile, segments=14, mat=mat_brass, parent=root, collection=col)

    primer_profile = [
        (-0.0285, 0.0),
        (-0.0285, 0.0030),
        (-0.0275, 0.0030),
        (-0.0275, 0.0),
    ]
    _create_lathe_mesh("CasingPrimer", primer_profile, segments=10, mat=mat_primer, parent=casing, collection=col)

    # Animación de expulsión: sale hacia la derecha (-Y), sube brevemente, cae hasta el suelo (-1.35m) y rebota
    act = bpy.data.actions.new("casing_eject")
    casing.animation_data_create()
    casing.animation_data.action = act

    # Física parabólica simulada a 60 fps (frames 1..38)
    fps = 60.0
    g = -9.81
    vx, vy, vz = -0.35, -1.65, 1.55
    floor_z = -1.34  # Altura relativa desde el arma hasta el suelo

    bounced = False
    bounce_t0 = 0.0
    bx, by, bz = 0.0, 0.0, 0.0
    bvx, bvy, bvz = 0.0, 0.0, 0.0

    for frame in range(1, 40):
        t = (frame - 1) / fps
        if not bounced:
            px = vx * t
            py = vy * t
            pz = vz * t + 0.5 * g * (t ** 2)
            if pz <= floor_z:
                bounced = True
                bounce_t0 = t
                bx, by, bz = px, py, floor_z
                bvx = vx * 0.45
                bvy = vy * 0.45
                # Velocidad de rebote hacia arriba
                vz_impact = vz + g * t
                bvz = abs(vz_impact) * 0.32
                pz = floor_z
        else:
            dt = t - bounce_t0
            px = bx + bvx * dt
            py = by + bvy * dt
            pz = bz + bvz * dt + 0.5 * g * (dt ** 2)
            if pz <= floor_z:
                pz = floor_z
                bvx *= 0.65
                bvy *= 0.65
                bvz = 0.0

        casing.location = (px, py, pz)
        # Rotación rápida en el aire que se estabiliza al tocar el suelo
        spin_factor = t if not bounced else (bounce_t0 + (t - bounce_t0) * 0.25)
        casing.rotation_euler = Euler(
            (
                spin_factor * 18.0,
                spin_factor * 24.0 if not bounced else math.radians(4.0),
                spin_factor * 14.0,
            ),
            "XYZ",
        )
        casing.keyframe_insert(data_path="location", frame=frame)
        casing.keyframe_insert(data_path="rotation_euler", frame=frame)

    push_action_to_nla(casing, act, "casing_eject", start_frame=1)
    bpy.context.scene.frame_start = 1
    bpy.context.scene.frame_end = 39
    bpy.context.scene.render.fps = 60

    if output_glb:
        export_glb(output_glb, export_animations=True)
    return root


if __name__ == "__main__":
    target = sys.argv[-1] if len(sys.argv) > 1 else ""
    if target.endswith("bullet_projectile.glb"):
        build_bullet_projectile(target)
    elif target.endswith("shell_casing_eject.glb"):
        build_shell_casing_eject(target)
    else:
        build_bullet_projectile("godot/assets/models/bullet_projectile.glb")
        build_shell_casing_eject("godot/assets/models/shell_casing_eject.glb")
