## Constructor visual modular para los personajes cápsula ("píldoras") y su rifle 7.62.
##
## Construye la malla estándar de cápsula 3D con visor táctico, banda de equipo y manos
## simples agarrando el arma; acopla `classic_assault_rifle.glb` rotado +90° en Y para
## alinear el cañón (glTF +X) hacia el frente de Godot (-Z) y conecta el fogonazo y FX
## al controlador `WeaponRig` en Rust.

class_name CapsuleVisualBuilder
extends RefCounted


static func build_capsule_and_weapon(
	character_root: Node3D,
	is_player: bool,
	assets: AssetLoader,
	fx_container: Node3D
) -> void:
	var visual_pivot := character_root.get_node_or_null("VisualPivot") as Node3D
	if visual_pivot != null:
		_populate_capsule_visuals(visual_pivot, is_player)

	var weapon_mount := character_root.get_node_or_null("WeaponMount") as Node3D
	if weapon_mount == null:
		return

	var weapon_rig := weapon_mount.get_node_or_null("WeaponRig")
	if weapon_rig == null:
		return

	# Limpia instancias previas si se reconstruye la escena.
	for child in weapon_rig.get_children():
		child.queue_free()

	var rifle_scene := assets.get_scene("rifle")
	if rifle_scene == null:
		return

	var rifle_inst := rifle_scene.instantiate() as Node3D
	if rifle_inst == null:
		return

	rifle_inst.name = "RifleModel"
	# En Blender/glTF el rifle apunta hacia +X; +90° en Y lo alinea con -Z (frente de Godot)
	# y sitúa el puerto de expulsión (glTF +Z) hacia +X (lado derecho).
	rifle_inst.rotation = Vector3(0.0, PI * 0.5, 0.0)
	rifle_inst.scale = Vector3(1.06, 1.06, 1.06)
	weapon_rig.add_child(rifle_inst)

	# Manos simples en cápsula sujetando empuñadura y guardamanos del rifle.
	_attach_capsule_hands(weapon_rig, is_player)

	var muzzle_pt := rifle_inst.find_child("MuzzlePoint", true, false) as Node3D
	var eject_pt := rifle_inst.find_child("EjectPoint", true, false) as Node3D
	var rifle_anim := _find_animation_player(rifle_inst)

	var flash_root: Node3D = null
	var flash_anim: AnimationPlayer = null
	var flash_scene := assets.get_scene("muzzle_flash")
	if flash_scene != null and muzzle_pt != null:
		var flash_inst := flash_scene.instantiate() as Node3D
		if flash_inst != null:
			flash_inst.name = "MuzzleFlashFX"
			flash_inst.visible = false
			muzzle_pt.add_child(flash_inst)
			flash_root = flash_inst
			flash_anim = _find_animation_player(flash_inst)

	weapon_rig.call("configure_parts", rifle_anim, muzzle_pt, eject_pt, flash_root, flash_anim)
	weapon_rig.call("set_fx_scenes", assets.get_scene("bullet"), assets.get_scene("casing"))
	weapon_rig.call("set_fx_container", fx_container)

	if character_root.has_method("set_grenade_scenes"):
		character_root.call(
			"set_grenade_scenes",
			assets.get_scene("grenade"),
			assets.get_scene("explosion"),
			fx_container
		)


static func _populate_capsule_visuals(visual_pivot: Node3D, is_player: bool) -> void:
	for child in visual_pivot.get_children():
		child.queue_free()

	var body_color := Color(0.24, 0.34, 0.48) if is_player else Color(0.42, 0.23, 0.20)
	var accent_color := Color(0.18, 0.72, 0.98) if is_player else Color(1.0, 0.36, 0.16)

	var body_mat := StandardMaterial3D.new()
	body_mat.albedo_color = body_color
	body_mat.roughness = 0.52
	body_mat.metallic = 0.12

	var capsule_mesh := CapsuleMesh.new()
	capsule_mesh.radius = 0.36
	capsule_mesh.height = 1.76
	capsule_mesh.radial_segments = 20
	capsule_mesh.rings = 6

	var body_mi := MeshInstance3D.new()
	body_mi.name = "CapsuleBody"
	body_mi.mesh = capsule_mesh
	body_mi.material_override = body_mat
	# Centro a 0.88m para que al escalar VisualPivot en Y al agacharse los pies sigan en y=0.
	body_mi.position = Vector3(0.0, 0.88, 0.0)
	visual_pivot.add_child(body_mi)

	# Visor frontal para distinguir claramente hacia dónde mira cada píldora.
	var visor_mat := StandardMaterial3D.new()
	visor_mat.albedo_color = accent_color
	visor_mat.emission_enabled = true
	visor_mat.emission = accent_color
	visor_mat.emission_energy_multiplier = 1.25
	visor_mat.roughness = 0.25

	var visor_mesh := BoxMesh.new()
	visor_mesh.size = Vector3(0.42, 0.14, 0.16)
	var visor_mi := MeshInstance3D.new()
	visor_mi.name = "TacticalVisor"
	visor_mi.mesh = visor_mesh
	visor_mi.material_override = visor_mat
	visor_mi.position = Vector3(0.0, 1.44, -0.31)
	visual_pivot.add_child(visor_mi)

	# Cinturón/banda táctica en el torso.
	var belt_mat := StandardMaterial3D.new()
	belt_mat.albedo_color = Color(0.11, 0.12, 0.14)
	belt_mat.roughness = 0.75
	var belt_mesh := CylinderMesh.new()
	belt_mesh.top_radius = 0.375
	belt_mesh.bottom_radius = 0.375
	belt_mesh.height = 0.16
	var belt_mi := MeshInstance3D.new()
	belt_mi.name = "TacticalBelt"
	belt_mi.mesh = belt_mesh
	belt_mi.material_override = belt_mat
	belt_mi.position = Vector3(0.0, 0.92, 0.0)
	visual_pivot.add_child(belt_mi)


static func _attach_capsule_hands(weapon_rig: Node3D, is_player: bool) -> void:
	var hand_mat := StandardMaterial3D.new()
	hand_mat.albedo_color = Color(0.16, 0.20, 0.26) if is_player else Color(0.24, 0.15, 0.14)
	hand_mat.roughness = 0.6

	var hand_mesh := SphereMesh.new()
	hand_mesh.radius = 0.065
	hand_mesh.height = 0.13

	var rear_hand := MeshInstance3D.new()
	rear_hand.name = "TriggerHand"
	rear_hand.mesh = hand_mesh
	rear_hand.material_override = hand_mat
	rear_hand.position = Vector3(0.0, -0.055, 0.072)
	weapon_rig.add_child(rear_hand)

	var front_hand := MeshInstance3D.new()
	front_hand.name = "SupportHand"
	front_hand.mesh = hand_mesh
	front_hand.material_override = hand_mat
	front_hand.position = Vector3(-0.025, -0.022, -0.22)
	weapon_rig.add_child(front_hand)


static func _find_animation_player(root: Node) -> AnimationPlayer:
	if root is AnimationPlayer:
		return root as AnimationPlayer
	for child in root.get_children():
		var found := _find_animation_player(child)
		if found != null:
			return found
	return null
