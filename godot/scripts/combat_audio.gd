## Gestor modular de efectos de sonido de combate (audio CC0 en formato OGG).
##
## Carga los efectos de disparo de rifle 7.62, recarga, caída de casquillos,
## pasos sobre concreto y aterrizaje; conecta tanto al jugador (audio directo 2D
## con polifonía para ráfagas rápidas) como a la IA enemiga (audio espacial 3D
## posicional para escuchar disparos y flanqueos por dirección).

class_name CombatAudio
extends Node

const AUDIO_PATHS := {
	"shot_1": "res://assets/audio/rifle_shot_1.ogg",
	"shot_2": "res://assets/audio/rifle_shot_2.ogg",
	"reload": "res://assets/audio/rifle_reload.ogg",
	"casing": "res://assets/audio/casing_drop.ogg",
	"land": "res://assets/audio/jump_land.ogg",
	"step_1": "res://assets/audio/footstep_1.ogg",
	"step_2": "res://assets/audio/footstep_2.ogg",
	"step_3": "res://assets/audio/footstep_3.ogg",
	"step_4": "res://assets/audio/footstep_4.ogg",
}

var _streams: Dictionary = {}
var _shot_toggle: bool = false
var _step_index: int = 0
var _rng := RandomNumberGenerator.new()

var _p_shot: AudioStreamPlayer
var _p_reload: AudioStreamPlayer
var _p_casing: AudioStreamPlayer
var _p_step: AudioStreamPlayer
var _p_land: AudioStreamPlayer

var _ai_shot_3d: AudioStreamPlayer3D
var _ai_reload_3d: AudioStreamPlayer3D
var _ai_step_3d: AudioStreamPlayer3D
var _ai_land_3d: AudioStreamPlayer3D


func _ready() -> void:
	_rng.randomize()
	_preload_streams()
	_setup_player_channels()


func _preload_streams() -> void:
	for key in AUDIO_PATHS.keys():
		var path: String = AUDIO_PATHS[key]
		if ResourceLoader.exists(path):
			var stream := load(path) as AudioStream
			if stream != null:
				_streams[key] = stream


func _setup_player_channels() -> void:
	_p_shot = AudioStreamPlayer.new()
	_p_shot.name = "PlayerShotAudio"
	_p_shot.max_polyphony = 8
	_p_shot.volume_db = -3.0
	add_child(_p_shot)

	_p_reload = AudioStreamPlayer.new()
	_p_reload.name = "PlayerReloadAudio"
	_p_reload.max_polyphony = 2
	_p_reload.volume_db = -2.0
	add_child(_p_reload)

	_p_casing = AudioStreamPlayer.new()
	_p_casing.name = "PlayerCasingAudio"
	_p_casing.max_polyphony = 6
	_p_casing.volume_db = -14.0
	add_child(_p_casing)

	_p_step = AudioStreamPlayer.new()
	_p_step.name = "PlayerStepAudio"
	_p_step.max_polyphony = 4
	_p_step.volume_db = -4.0
	add_child(_p_step)

	_p_land = AudioStreamPlayer.new()
	_p_land.name = "PlayerLandAudio"
	_p_land.max_polyphony = 2
	_p_land.volume_db = -4.0
	add_child(_p_land)


func attach_to_combatants(player_node: Node3D, enemy_node: Node3D) -> void:
	if player_node != null:
		if not player_node.is_connected("footstep_triggered", _on_player_footstep):
			player_node.connect("footstep_triggered", _on_player_footstep)
		if not player_node.is_connected("landed", _on_player_landed):
			player_node.connect("landed", _on_player_landed)

		var p_rig := player_node.call("get_weapon_rig") as Node
		if p_rig != null:
			if not p_rig.is_connected("weapon_fired", _on_player_weapon_fired):
				p_rig.connect("weapon_fired", _on_player_weapon_fired)
			if not p_rig.is_connected("reload_started", _on_player_reload_started):
				p_rig.connect("reload_started", _on_player_reload_started)

	if enemy_node != null:
		_ensure_enemy_audio_nodes(enemy_node)
		if not enemy_node.is_connected("footstep_triggered", _on_enemy_footstep):
			enemy_node.connect("footstep_triggered", _on_enemy_footstep)
		if not enemy_node.is_connected("landed", _on_enemy_landed):
			enemy_node.connect("landed", _on_enemy_landed)

		var ai_rig := enemy_node.call("get_weapon_rig") as Node
		if ai_rig != null:
			if not ai_rig.is_connected("weapon_fired", _on_enemy_weapon_fired):
				ai_rig.connect("weapon_fired", _on_enemy_weapon_fired)
			if not ai_rig.is_connected("reload_started", _on_enemy_reload_started):
				ai_rig.connect("reload_started", _on_enemy_reload_started)


func _ensure_enemy_audio_nodes(enemy_node: Node3D) -> void:
	if _ai_shot_3d != null and is_instance_valid(_ai_shot_3d):
		return

	_ai_shot_3d = _make_3d_player("EnemyShot3D", 6, 9.0, 56.0, -1.5)
	enemy_node.add_child(_ai_shot_3d)

	_ai_reload_3d = _make_3d_player("EnemyReload3D", 2, 5.0, 28.0, -4.0)
	enemy_node.add_child(_ai_reload_3d)

	_ai_step_3d = _make_3d_player("EnemyStep3D", 4, 4.5, 26.0, -3.5)
	enemy_node.add_child(_ai_step_3d)

	_ai_land_3d = _make_3d_player("EnemyLand3D", 2, 5.0, 28.0, -3.0)
	enemy_node.add_child(_ai_land_3d)


func _make_3d_player(node_name: String, polyphony: int, unit_sz: float, max_dist: float, vol_db: float) -> AudioStreamPlayer3D:
	var p := AudioStreamPlayer3D.new()
	p.name = node_name
	p.max_polyphony = polyphony
	p.unit_size = unit_sz
	p.max_distance = max_dist
	p.volume_db = vol_db
	p.attenuation_model = AudioStreamPlayer3D.ATTENUATION_INVERSE_DISTANCE
	return p


func set_sfx_volume(ratio: float) -> void:
	var clamped := clampf(ratio, 0.0, 1.0)
	var master_idx := AudioServer.get_bus_index("Master")
	if clamped <= 0.01:
		AudioServer.set_bus_mute(master_idx, true)
	else:
		AudioServer.set_bus_mute(master_idx, false)
		AudioServer.set_bus_volume_db(master_idx, linear_to_db(clamped))


func _next_shot_stream() -> AudioStream:
	_shot_toggle = not _shot_toggle
	var key := "shot_1" if _shot_toggle else "shot_2"
	return _streams.get(key, _streams.get("shot_1", null)) as AudioStream


func _next_step_stream() -> AudioStream:
	_step_index = (_step_index + 1) % 4
	var key := "step_%d" % (_step_index + 1)
	return _streams.get(key, _streams.get("step_1", null)) as AudioStream


func _on_player_weapon_fired() -> void:
	var s := _next_shot_stream()
	if s != null:
		_p_shot.stream = s
		_p_shot.pitch_scale = _rng.randf_range(0.95, 1.06)
		_p_shot.play()
	_schedule_player_casing_sound()


func _schedule_player_casing_sound() -> void:
	var casing_stream := _streams.get("casing", null) as AudioStream
	if casing_stream == null or not is_inside_tree():
		return
	var tree := get_tree()
	if tree == null:
		return
	await tree.create_timer(0.28).timeout
	if not is_instance_valid(_p_casing):
		return
	_p_casing.stream = casing_stream
	_p_casing.pitch_scale = _rng.randf_range(0.92, 1.12)
	_p_casing.play()


func _on_player_reload_started(_duration: float) -> void:
	var s := _streams.get("reload", null) as AudioStream
	if s != null:
		_p_reload.stream = s
		_p_reload.pitch_scale = _rng.randf_range(0.98, 1.03)
		_p_reload.play()


func _on_player_footstep(is_sprinting: bool, is_crouching: bool) -> void:
	var s := _next_step_stream()
	if s == null:
		return
	_p_step.stream = s
	if is_crouching:
		_p_step.volume_db = -10.5
		_p_step.pitch_scale = _rng.randf_range(0.86, 0.96)
	elif is_sprinting:
		_p_step.volume_db = -1.5
		_p_step.pitch_scale = _rng.randf_range(1.02, 1.12)
	else:
		_p_step.volume_db = -4.5
		_p_step.pitch_scale = _rng.randf_range(0.94, 1.05)
	_p_step.play()


func _on_player_landed() -> void:
	var s := _streams.get("land", null) as AudioStream
	if s != null:
		_p_land.stream = s
		_p_land.pitch_scale = _rng.randf_range(0.95, 1.05)
		_p_land.play()


func _on_enemy_weapon_fired() -> void:
	if _ai_shot_3d == null or not is_instance_valid(_ai_shot_3d):
		return
	var s := _next_shot_stream()
	if s != null:
		_ai_shot_3d.stream = s
		_ai_shot_3d.pitch_scale = _rng.randf_range(0.93, 1.04)
		_ai_shot_3d.play()


func _on_enemy_reload_started(_duration: float) -> void:
	if _ai_reload_3d == null or not is_instance_valid(_ai_reload_3d):
		return
	var s := _streams.get("reload", null) as AudioStream
	if s != null:
		_ai_reload_3d.stream = s
		_ai_reload_3d.play()


func _on_enemy_footstep(is_sprinting: bool, is_crouching: bool) -> void:
	if _ai_step_3d == null or not is_instance_valid(_ai_step_3d):
		return
	var s := _next_step_stream()
	if s == null:
		return
	_ai_step_3d.stream = s
	if is_crouching:
		_ai_step_3d.volume_db = -11.0
	elif is_sprinting:
		_ai_step_3d.volume_db = -1.5
	else:
		_ai_step_3d.volume_db = -4.5
	_ai_step_3d.pitch_scale = _rng.randf_range(0.92, 1.08)
	_ai_step_3d.play()


func _on_enemy_landed() -> void:
	if _ai_land_3d == null or not is_instance_valid(_ai_land_3d):
		return
	var s := _streams.get("land", null) as AudioStream
	if s != null:
		_ai_land_3d.stream = s
		_ai_land_3d.play()
