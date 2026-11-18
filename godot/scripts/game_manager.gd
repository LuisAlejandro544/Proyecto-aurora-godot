## Orquestador principal de Iron Vector (`main.tscn`).
##
## Coordina el ciclo completo entre pantallas separadas:
## 1. Menú Principal (cámara orbital 3D sobre la ciudad destruida una vez cargada).
## 2. Pantalla de Carga asíncrona (descarga de escena/modelos GLB + NavMesh en segundo plano).
## 3. Combate Activo por Rondas (Jugador cápsula vs IA local inteligente con AK-47).
## 4. Pausa y Pantalla de Victoria/Derrota.

class_name GameManager
extends Node

@onready var asset_loader: AssetLoader = $AssetLoader
@onready var arena: ArenaController = $Arena
@onready var player: Node3D = $PlayerCapsule
@onready var enemy_ai: Node3D = $EnemyAiCapsule
@onready var menu_camera_pivot: Node3D = $MenuCameraPivot
@onready var menu_camera: Camera3D = $MenuCameraPivot/MenuCamera

@onready var ui_menu: MenuScreen = $UI_Menu
@onready var ui_loading: Control = $UI_Loading
@onready var loading_bar: ProgressBar = $UI_Loading/Panel/VBox/LoadingBar
@onready var loading_label: Label = $UI_Loading/Panel/VBox/LoadingLabel
@onready var ui_hud: TouchHud = $UI_HUD
@onready var ui_end_overlay: EndOverlay = $UI_EndOverlay

var _assets_ready: bool = false
var _in_match: bool = false
var _is_paused: bool = false
var _first_person: bool = false
var _difficulty: int = 1
var _rounds_to_win: int = 2
var _current_round: int = 1
var _player_score: int = 0
var _ai_score: int = 0
var _round_hits: int = 0
var _total_hits: int = 0
var _sensitivity_mult: float = 1.0


func _ready() -> void:
	ui_menu.visible = false
	ui_hud.visible = false
	ui_end_overlay.visible = false
	ui_loading.visible = true

	player.visible = false
	enemy_ai.visible = false
	menu_camera.current = true

	_connect_ui_signals()
	_connect_combat_signals()

	# Carga inicial asíncrona mediante corrutinas para tener el escenario listo como fondo vivo.
	var ok := await _bootstrap_assets_and_world()
	if ok:
		ui_loading.visible = false
		ui_menu.visible = true


func _process(delta: float) -> void:
	if not _in_match and menu_camera.current:
		menu_camera_pivot.rotate_y(delta * 0.14)


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("ui_cancel") and _in_match and not ui_end_overlay.visible:
		_open_pause()


func _bootstrap_assets_and_world() -> bool:
	asset_loader.progress_updated.connect(func(ratio: float, msg: String):
		loading_bar.value = ratio * 100.0
		loading_label.text = msg
	)

	var ok := await asset_loader.load_all_assets()
	if not ok:
		loading_label.text = "Error: faltan modelos .glb en res://assets/models/.\nDescárgalos desde el Release de GitHub."
		return false

	loading_label.text = "Construyendo ciudad destruida y malla de navegación IA..."
	loading_bar.value = 92.0
	var arena_ok := await arena.build_arena(asset_loader)
	if not arena_ok:
		loading_label.text = "Error instanciando ruined_city_arena.glb"
		return false

	CapsuleVisualBuilder.build_capsule_and_weapon(player, true, asset_loader, arena.fx_container)
	CapsuleVisualBuilder.build_capsule_and_weapon(enemy_ai, false, asset_loader, arena.fx_container)

	enemy_ai.call("set_player_node", player)
	enemy_ai.call("clear_cover_points")
	for pt in arena.cover_points:
		enemy_ai.call("register_cover_point", pt)

	_connect_weapon_hud_signals()
	_assets_ready = true
	loading_bar.value = 100.0
	return true


func _connect_ui_signals() -> void:
	ui_menu.deploy_requested.connect(_on_deploy_requested)
	ui_menu.settings_changed.connect(func(sens: float, inv_y: bool):
		_sensitivity_mult = sens
		ui_hud.invert_y = inv_y
		player.call("set_aim_sensitivity", 0.0032 * sens)
	)

	ui_hud.move_input_changed.connect(func(vec: Vector2): player.call("set_move_input", vec))
	ui_hud.sprint_changed.connect(func(sp: bool): player.call("set_sprinting", sp))
	ui_hud.look_delta_input.connect(func(d: Vector2): player.call("add_look_delta", d))
	ui_hud.fire_state_changed.connect(func(f: bool): player.call("set_firing", f))
	ui_hud.reload_pressed.connect(func(): player.call("trigger_reload"))
	ui_hud.crouch_toggled.connect(func(c: bool): player.call("set_crouching", c))
	ui_hud.jump_pressed.connect(func(): player.call("trigger_jump"))
	ui_hud.camera_toggle_pressed.connect(_toggle_camera_perspective)
	ui_hud.pause_pressed.connect(_open_pause)

	ui_end_overlay.resume_pressed.connect(_resume_from_pause)
	ui_end_overlay.next_round_pressed.connect(_start_next_round)
	ui_end_overlay.rematch_pressed.connect(func():
		_on_deploy_requested(_difficulty, _first_person, _rounds_to_win)
	)
	ui_end_overlay.return_menu_pressed.connect(_return_to_menu)
	ui_end_overlay.quick_sensitivity_changed.connect(func(sens: float):
		_sensitivity_mult = sens
		player.call("set_aim_sensitivity", 0.0032 * sens)
	)
	ui_end_overlay.quick_camera_toggled.connect(_toggle_camera_perspective)


func _connect_combat_signals() -> void:
	player.connect("health_changed", func(cur: float, mx: float):
		ui_hud.update_player_health(cur, mx)
	)
	player.connect("hit_confirmed", func(_dmg: float):
		_round_hits += 1
		_total_hits += 1
		ui_hud.notify_hit_confirmed()
	)
	player.connect("took_damage", func(_amt: float, _pos: Vector3):
		ui_hud.notify_took_damage()
	)
	player.connect("died", _on_player_died)

	enemy_ai.connect("health_changed", func(cur: float, mx: float):
		ui_hud.update_enemy_health(cur, mx)
	)
	enemy_ai.connect("state_changed", func(state_name: String):
		ui_hud.update_enemy_state(state_name)
	)
	enemy_ai.connect("died", _on_enemy_died)


func _connect_weapon_hud_signals() -> void:
	var p_rig := player.call("get_weapon_rig") as Node
	if p_rig == null:
		return
	p_rig.connect("ammo_changed", func(in_mag: int, res: int): ui_hud.update_ammo(in_mag, res))
	p_rig.connect("reload_started", func(dur: float): ui_hud.notify_reload_started(dur))
	p_rig.connect("reload_finished", func(): ui_hud.notify_reload_finished())
	p_rig.connect("weapon_fired", func(): ui_hud.notify_weapon_fired())


func _on_deploy_requested(difficulty: int, first_person: bool, rounds_to_win: int) -> void:
	if not _assets_ready:
		return
	_difficulty = difficulty
	_first_person = first_person
	_rounds_to_win = rounds_to_win
	_player_score = 0
	_ai_score = 0
	_current_round = 1
	_total_hits = 0

	enemy_ai.call("set_difficulty", _difficulty)
	player.call("set_camera_mode_first_person", _first_person)

	ui_menu.visible = false
	ui_end_overlay.visible = false
	ui_hud.visible = true
	_start_round()


func _start_round() -> void:
	_in_match = true
	_is_paused = false
	_round_hits = 0
	arena.clear_dynamic_fx()

	player.visible = true
	enemy_ai.visible = true

	var p_spawn := arena.spawn_player_pos
	var ai_spawn := arena.spawn_ai_pos
	player.call("reset_for_round", p_spawn, ai_spawn)
	enemy_ai.call("reset_for_round", ai_spawn, p_spawn)

	var p_cam := player.get_node_or_null("CameraPivot/SpringArm3D/Camera3D") as Camera3D
	if p_cam != null:
		p_cam.current = true

	ui_hud.reset_hud_for_round(_player_score, _ai_score, _current_round, _rounds_to_win)
	player.call("set_controls_enabled", true)
	enemy_ai.call("set_ai_active", true)

	if not DisplayServer.is_touchscreen_available():
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED


func _on_player_died() -> void:
	if not _in_match:
		return
	_in_match = false
	_ai_score += 1
	_finish_round(false)


func _on_enemy_died() -> void:
	if not _in_match:
		return
	_in_match = false
	_player_score += 1
	_finish_round(true)


func _finish_round(player_won: bool) -> void:
	player.call("set_controls_enabled", false)
	enemy_ai.call("set_ai_active", false)
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE

	await get_tree().create_timer(0.85).timeout
	ui_hud.visible = false

	if _player_score >= _rounds_to_win or _ai_score >= _rounds_to_win:
		ui_end_overlay.show_match_result(_player_score > _ai_score, _player_score, _ai_score, _total_hits)
	else:
		ui_end_overlay.show_round_result(player_won, _player_score, _ai_score, _round_hits)


func _start_next_round() -> void:
	_current_round += 1
	ui_end_overlay.visible = false
	ui_hud.visible = true
	_start_round()


func _open_pause() -> void:
	_is_paused = true
	player.call("set_controls_enabled", false)
	enemy_ai.call("set_ai_active", false)
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	ui_end_overlay.show_pause(_player_score, _ai_score, _sensitivity_mult)


func _resume_from_pause() -> void:
	_is_paused = false
	ui_end_overlay.visible = false
	player.call("set_controls_enabled", true)
	enemy_ai.call("set_ai_active", true)
	if not DisplayServer.is_touchscreen_available():
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED


func _toggle_camera_perspective() -> void:
	_first_person = not _first_person
	player.call("set_camera_mode_first_person", _first_person)


func _return_to_menu() -> void:
	_in_match = false
	_is_paused = false
	player.call("set_controls_enabled", false)
	enemy_ai.call("set_ai_active", false)
	player.visible = false
	enemy_ai.visible = false
	arena.clear_dynamic_fx()

	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	menu_camera.current = true
	ui_end_overlay.visible = false
	ui_hud.visible = false
	ui_menu.visible = true
