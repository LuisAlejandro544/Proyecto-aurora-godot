"""
Utilidades compartidas para los scripts de Blender en modo headless (sin interfaz).
Incluye limpieza de escena, creación de materiales PBR compatibles con Blender 4/5
y Godot 4.7, helpers geométricos con bmesh, biselado hard-surface y exportación a GLB.
"""

import math
import os
import bpy
import bmesh
from mathutils import Euler, Matrix, Vector


def clear_scene():
    """Elimina todos los objetos, mallas, materiales, luces, cámaras y acciones actuales."""
    if bpy.context.active_object and bpy.context.active_object.mode != "OBJECT":
        bpy.ops.object.mode_set(mode="OBJECT")

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)

    for collection in (
        bpy.data.meshes,
        bpy.data.materials,
        bpy.data.lights,
        bpy.data.cameras,
        bpy.data.actions,
        bpy.data.images,
    ):
        for item in list(collection):
            if item.users == 0:
                collection.remove(item)
        # Segunda pasada para asegurar limpieza total
        for item in list(collection):
            collection.remove(item)


def get_or_create_collection(name, parent=None):
    """Obtiene o crea una colección en la escena para mantener el archivo ordenado."""
    if name in bpy.data.collections:
        return bpy.data.collections[name]
    col = bpy.data.collections.new(name)
    if parent is None:
        bpy.context.scene.collection.children.link(col)
    else:
        parent.children.link(col)
    return col


def link_to_collection(obj, collection):
    """Mueve un objeto exclusivamente a la colección indicada."""
    for c in list(obj.users_collection):
        c.objects.unlink(obj)
    collection.objects.link(obj)


def make_pbr_material(
    name,
    base_color=(0.5, 0.5, 0.5, 1.0),
    metallic=0.0,
    roughness=0.5,
    emission_color=(0.0, 0.0, 0.0, 1.0),
    emission_strength=0.0,
    alpha=1.0,
):
    """Crea un material Principled BSDF ligero y compatible con exportación glTF/Godot."""
    if name in bpy.data.materials:
        return bpy.data.materials[name]

    mat = bpy.data.materials.new(name=name)
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    bsdf = nodes.get("Principled BSDF")
    if bsdf is None:
        bsdf = nodes.new(type="ShaderNodeBsdfPrincipled")

    bsdf.inputs["Base Color"].default_value = base_color
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness

    if "Emission Color" in bsdf.inputs:
        bsdf.inputs["Emission Color"].default_value = emission_color
    if "Emission Strength" in bsdf.inputs:
        bsdf.inputs["Emission Strength"].default_value = emission_strength
    if "Alpha" in bsdf.inputs:
        bsdf.inputs["Alpha"].default_value = alpha

    if alpha < 1.0 and hasattr(mat, "blend_method"):
        mat.blend_method = "BLEND"

    return mat


def assign_material(obj, mat):
    """Asigna un material único a un objeto de malla."""
    if obj.data.materials:
        obj.data.materials[0] = mat
    else:
        obj.data.materials.append(mat)


def set_smooth_shading(obj, smooth=True):
    """Activa o desactiva el sombreado suave por polígono de forma compatible con Blender 4/5."""
    mesh = obj.data
    if not mesh or not hasattr(mesh, "polygons"):
        return
    vals = [smooth] * len(mesh.polygons)
    mesh.polygons.foreach_set("use_smooth", vals)
    mesh.update()


def add_bevel(obj, width=0.004, segments=2):
    """Aplica un modificador Bevel para bordes hard-surface realistas y mejores reflejos."""
    mod = obj.modifiers.new(name="Bevel", type="BEVEL")
    mod.width = width
    mod.segments = segments
    mod.limit_method = "ANGLE"
    mod.angle_limit = math.radians(32.0)
    mod.harden_normals = False
    return mod


def create_box(
    name,
    size=(1.0, 1.0, 1.0),
    location=(0.0, 0.0, 0.0),
    rotation=(0.0, 0.0, 0.0),
    material=None,
    bevel_width=0.0,
    collection=None,
):
    """Crea una caja parametrizada con bmesh y origen centrado."""
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.scale(bm, vec=size, verts=bm.verts)
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    obj.location = location
    obj.rotation_euler = Euler(rotation, "XYZ")
    bpy.context.scene.collection.objects.link(obj)
    if collection is not None:
        link_to_collection(obj, collection)
    if material is not None:
        assign_material(obj, material)
    if bevel_width > 0.0:
        add_bevel(obj, width=bevel_width, segments=2)
    return obj


def create_cylinder(
    name,
    radius=0.05,
    depth=0.5,
    segments=16,
    location=(0.0, 0.0, 0.0),
    rotation=(0.0, 0.0, 0.0),
    material=None,
    smooth=True,
    collection=None,
):
    """Crea un cilindro alineado inicialmente en Z (rotar Y=90° para alinear en +X)."""
    mesh = bpy.data.meshes.new(name + "_Mesh")
    bm = bmesh.new()
    bmesh.ops.create_cone(
        bm,
        cap_ends=True,
        cap_tris=False,
        segments=segments,
        radius1=radius,
        radius2=radius,
        depth=depth,
    )
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    obj.location = location
    obj.rotation_euler = Euler(rotation, "XYZ")
    bpy.context.scene.collection.objects.link(obj)
    if collection is not None:
        link_to_collection(obj, collection)
    if material is not None:
        assign_material(obj, material)
    set_smooth_shading(obj, smooth=smooth)
    return obj


def create_empty(name, location=(0.0, 0.0, 0.0), rotation=(0.0, 0.0, 0.0), parent=None, collection=None):
    """Crea un nodo Empty usado como punto de anclaje (MuzzlePoint, EjectPoint, Spawns)."""
    empty = bpy.data.objects.new(name, None)
    empty.empty_display_type = "PLAIN_AXES"
    empty.empty_display_size = 0.08
    empty.location = location
    empty.rotation_euler = Euler(rotation, "XYZ")
    bpy.context.scene.collection.objects.link(empty)
    if collection is not None:
        link_to_collection(empty, collection)
    if parent is not None:
        empty.parent = parent
    return empty


def push_action_to_nla(obj, action, track_name, start_frame=1):
    """Empuja una Action al NLA Track del objeto para exportar múltiples animaciones limpias en GLB."""
    if obj.animation_data is None:
        obj.animation_data_create()
    track = obj.animation_data.nla_tracks.new()
    track.name = track_name
    strip = track.strips.new(action.name, int(start_frame), action)
    strip.name = action.name
    obj.animation_data.action = None
    return track


def export_glb(filepath, export_animations=True):
    """Exporta la escena actual a formato GLB binario listo para Godot 4.7."""
    os.makedirs(os.path.dirname(os.path.abspath(filepath)), exist_ok=True)
    kwargs = dict(
        filepath=os.path.abspath(filepath),
        export_format="GLB",
        use_selection=False,
        export_apply=True,
        export_animations=export_animations,
        export_yup=True,
    )
    # Compatibilidad segura de parámetros según versión de glTF exporter en Blender 4/5
    op_props = bpy.ops.export_scene.gltf.get_rna_type().properties
    if "export_animation_mode" in op_props:
        kwargs["export_animation_mode"] = "NLA_TRACKS" if export_animations else "ACTIONS"
    if "export_nla_strips" in op_props:
        kwargs["export_nla_strips"] = True
    if "export_nla_strips_merged_animation_name" in op_props:
        kwargs["export_nla_strips_merged_animation_name"] = "Animation"

    bpy.ops.export_scene.gltf(**kwargs)
    print(f"[Blender Export] GLB guardado en: {filepath}")
