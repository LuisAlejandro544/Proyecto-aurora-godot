## Controlador del escenario de ciudad destruida (`ruined_city_arena.glb`).
##
## Instancia la ciudad destruida dentro de `NavigationRegion3D`, genera de forma asíncrona
## la malla de navegación para que la IA esquive edificios y escombros, localiza los puntos
## de aparición (`SpawnPlayer` / `SpawnAI`) y calcula posiciones tácticas de cobertura reales
## detrás de barreras Jersey, sacos de arena, autos calcinados y cajas.

class_name ArenaController
extends Node3D

@onready var nav_region: NavigationRegion3D = $NavigationRegion3D
@onready var geometry_holder: Node3D = $NavigationRegion3D/GeometryHolder
@onready var fx_container: Node3D = $FxContainer

var spawn_player_pos: Vector3 = Vector3(0.0, 0.1, 16.8)
var spawn_ai_pos: Vector3 = Vector3(0.0, 0.1, -16.8)
var cover_points: Array[Vector3] = []
var _is_built: bool = false


func build_arena(assets: AssetLoader) -> bool:
	if _is_built:
		return true

	for child in geometry_holder.get_children():
		child.queue_free()

	var arena_scene := assets.get_scene("arena")
	if arena_scene == null:
		return false

	var arena_inst := arena_scene.instantiate() as Node3D
	if arena_inst == null:
		return false

	arena_inst.name = "RuinedCityInstance"
	geometry_holder.add_child(arena_inst)

	# Localiza los nodos Empty generados por Blender para aparición de jugador e IA.
	var sp_player := arena_inst.find_child("SpawnPlayer", true, false) as Node3D
	if sp_player != null:
		spawn_player_pos = sp_player.global_position + Vector3(0.0, 0.08, 0.0)
	var sp_ai := arena_inst.find_child("SpawnAI", true, false) as Node3D
	if sp_ai != null:
		spawn_ai_pos = sp_ai.global_position + Vector3(0.0, 0.08, 0.0)

	_extract_tactical_cover_points(arena_inst)

	# Espera dos frames para que los StaticBody3D generados por `-col` entren al árbol físico.
	await get_tree().physics_frame
	await get_tree().physics_frame

	await _bake_navigation_async()
	_is_built = true
	return true


func clear_dynamic_fx() -> void:
	for child in fx_container.get_children():
		child.queue_free()


func _bake_navigation_async() -> void:
	var nav_mesh := NavigationMesh.new()
	nav_mesh.agent_radius = 0.45
	nav_mesh.agent_height = 1.75
	nav_mesh.agent_max_climb = 0.42
	nav_mesh.agent_max_slope = 42.0
	nav_mesh.cell_size = 0.25
	nav_mesh.cell_height = 0.20
	nav_mesh.geometry_parsed_geometry_type = NavigationMesh.PARSED_GEOMETRY_STATIC_COLLIDERS
	nav_mesh.geometry_source_geometry_mode = NavigationMesh.SOURCE_GEOMETRY_ROOT_NODE_CHILDREN
	nav_region.navigation_mesh = nav_mesh

	nav_region.bake_navigation_mesh(true)
	await nav_region.bake_finished


func _extract_tactical_cover_points(arena_inst: Node3D) -> void:
	cover_points.clear()
	# Puntos curados en torno a barreras, trincheras, vehículos y flancos de la ciudad (espacio Godot Y-up).
	# Conversión desde Blender: Godot (X, Y, Z) = Blender (X, Z, -Y).
	var anchors := [
		Vector3(-2.6, 0.1, -3.5),
		Vector3(-2.6, 0.1, -0.9),
		Vector3(2.8, 0.1, 0.7),
		Vector3(2.8, 0.1, 3.3),
		Vector3(-1.5, 0.1, -10.1),
		Vector3(-1.5, 0.1, -7.5),
		Vector3(1.6, 0.1, -10.9),
		Vector3(1.5, 0.1, 7.5),
		Vector3(-1.6, 0.1, 10.9),
		Vector3(9.0, 0.1, -0.3),
		Vector3(9.0, 0.1, -2.9),
		Vector3(-9.0, 0.1, 0.3),
		Vector3(-9.0, 0.1, 2.9),
		Vector3(-4.8, 0.1, -0.6),
		Vector3(-4.8, 0.1, 2.2),
		Vector3(5.2, 0.1, -2.8),
		Vector3(5.2, 0.1, 0.0),
		Vector3(0.8, 0.1, -15.4),
		Vector3(0.0, 0.1, 15.4),
		Vector3(14.1, 0.1, 0.0),
		Vector3(-14.1, 0.1, 0.0),
		Vector3(3.8, 0.1, -6.9),
		Vector3(-4.2, 0.1, 7.0),
		Vector3(-6.2, 0.1, -6.1),
		Vector3(6.2, 0.1, 6.1),
		Vector3(7.4, 0.1, -7.6),
		Vector3(-7.4, 0.1, 7.6),
	]
	for pt in anchors:
		cover_points.append(pt)

	# Añade también puntos dinámicos si el escenario incluye nodos adicionales con prefijos clave.
	for child in arena_inst.find_children("*", "Node3D", true, false):
		var n := child.name
		if n.begins_with("Sandbags_") or n.begins_with("Barrier_"):
			var p: Vector3 = (child as Node3D).global_position
			p.y = 0.1
			var fwd := -(child as Node3D).global_transform.basis.z
			fwd.y = 0.0
			if fwd.length_squared() > 0.01:
				fwd = fwd.normalized()
				cover_points.append(p + fwd * 1.15)
				cover_points.append(p - fwd * 1.15)
