## Interfaz de combate móvil (HUD táctico + controles multitáctiles).
##
## Implementa joystick virtual izquierdo con sprint en borde exterior, zona derecha de
## apuntado por arrastre táctil sin bloquear botones, botones de disparo, recarga, agachado,
## salto, cambio de cámara y pausa, además de mira dinámica, hitmarker y marcador de rondas.

class_name TouchHud
extends Control

signal move_input_changed(vec: Vector2)
signal sprint_changed(sprinting: bool)
signal look_delta_input(delta: Vector2)
signal fire_state_changed(firing: bool)
signal reload_pressed()
signal crouch_toggled(crouching: bool)
signal jump_pressed()
signal camera_toggle_pressed()
signal pause_pressed()

@onready var player_hp_bar: ProgressBar = %PlayerHpBar
@onready var player_hp_label: Label = %PlayerHpLabel
@onready var stance_label: Label = %StanceLabel

@onready var score_label: Label = %ScoreLabel
@onready var round_label: Label = %RoundLabel

@onready var enemy_hp_bar: ProgressBar = %EnemyHpBar
@onready var enemy_state_label: Label = %EnemyStateLabel

@onready var ammo_label: Label = %AmmoLabel
@onready var reload_bar: ProgressBar = %ReloadBar

@onready var crosshair_root: Control = %CrosshairRoot
@onready var tick_top: ColorRect = %TickTop
@onready var tick_bottom: ColorRect = %TickBottom
@onready var tick_left: ColorRect = %TickLeft
@onready var tick_right: ColorRect = %TickRight
@onready var hitmarker: Control = %Hitmarker
@onready var damage_vignette: ColorRect = %DamageVignette

@onready var joystick_area: Control = %JoystickArea
@onready var joystick_base: Control = %JoystickBase
@onready var joystick_knob: Control = %JoystickKnob

@onready var btn_fire: Button = %BtnFire
@onready var btn_reload: Button = %BtnReload
@onready var btn_crouch: Button = %BtnCrouch
@onready var btn_jump: Button = %BtnJump
@onready var btn_camera: Button = %BtnCamera
@onready var btn_pause: Button = %BtnPause

var invert_y: bool = false
var _joy_touch_index: int = -1
var _look_touch_index: int = -1
var _joy_center: Vector2 = Vector2.ZERO
var _joy_radius: float = 68.0
var _is_crouching: bool = false
var _is_reloading: bool = false
var _reload_duration: float = 2.3
var _reload_elapsed: float = 0.0
var _hitmarker_timer: float = 0.0
var _damage_flash_timer: float = 0.0
var _crosshair_spread_px: float = 10.0


func _ready() -> void:
	hitmarker.visible = false
	damage_vignette.modulate.a = 0.0
	reload_bar.visible = false

	btn_fire.button_down.connect(func(): fire_state_changed.emit(true))
	btn_fire.button_up.connect(func(): fire_state_changed.emit(false))
	btn_reload.pressed.connect(func(): reload_pressed.emit())
	btn_crouch.pressed.connect(_on_crouch_pressed)
	btn_jump.pressed.connect(func(): jump_pressed.emit())
	btn_camera.pressed.connect(func(): camera_toggle_pressed.emit())
	btn_pause.pressed.connect(func(): pause_pressed.emit())

	_update_joystick_center()


func _process(delta: float) -> void:
	if _hitmarker_timer > 0.0:
		_hitmarker_timer -= delta
		if _hitmarker_timer <= 0.0:
			hitmarker.visible = false

	if _damage_flash_timer > 0.0:
		_damage_flash_timer = max(0.0, _damage_flash_timer - delta)
		damage_vignette.modulate.a = (_damage_flash_timer / 0.35) * 0.55

	if _is_reloading:
		_reload_elapsed += delta
		reload_bar.value = clampf(_reload_elapsed / max(_reload_duration, 0.1), 0.0, 1.0)

	# Anima suavemente la apertura de los 4 trazos de la mira.
	var target_gap := 6.0 if _is_crouching else 11.0
	_crosshair_spread_px = lerpf(_crosshair_spread_px, target_gap, minf(delta * 10.0, 1.0))
	tick_top.position.y = -_crosshair_spread_px - tick_top.size.y
	tick_bottom.position.y = _crosshair_spread_px
	tick_left.position.x = -_crosshair_spread_px - tick_left.size.x
	tick_right.position.x = _crosshair_spread_px


func _input(event: InputEvent) -> void:
	if not visible:
		return

	if event is InputEventScreenTouch:
		var st := event as InputEventScreenTouch
		if st.pressed:
			if _joy_touch_index == -1 and joystick_area.get_global_rect().has_point(st.position):
				_joy_touch_index = st.index
				_update_joystick_center()
				_apply_joystick_drag(st.position)
			elif _look_touch_index == -1 and not _is_over_action_button(st.position):
				if st.position.x >= get_viewport_rect().size.x * 0.34:
					_look_touch_index = st.index
		else:
			if st.index == _joy_touch_index:
				_reset_joystick()
			elif st.index == _look_touch_index:
				_look_touch_index = -1

	elif event is InputEventScreenDrag:
		var sd := event as InputEventScreenDrag
		if sd.index == _joy_touch_index:
			_apply_joystick_drag(sd.position)
		elif sd.index == _look_touch_index:
			var d := sd.relative
			if invert_y:
				d.y = -d.y
			look_delta_input.emit(d)

	elif event is InputEventMouseMotion:
		if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
			var mm := event as InputEventMouseMotion
			var d := mm.relative
			if invert_y:
				d.y = -d.y
			look_delta_input.emit(d)


func reset_hud_for_round(p_score: int, ai_score: int, current_round: int, rounds_to_win: int) -> void:
	_reset_joystick()
	_look_touch_index = -1
	_is_crouching = false
	_is_reloading = false
	reload_bar.visible = false
	hitmarker.visible = false
	damage_vignette.modulate.a = 0.0
	stance_label.text = "DE PIE"
	btn_crouch.modulate = Color.WHITE
	update_scoreboard(p_score, ai_score, current_round, rounds_to_win)
	update_player_health(100.0, 100.0)
	update_enemy_health(100.0, 100.0)
	update_ammo(30, 120)
	update_enemy_state("Avanzando")


func update_scoreboard(p_score: int, ai_score: int, current_round: int, rounds_to_win: int) -> void:
	score_label.text = "TÚ %d  :  %d IA" % [p_score, ai_score]
	round_label.text = "RONDA %d (META %d)" % [current_round, rounds_to_win]


func update_player_health(current: float, max_hp: float) -> void:
	player_hp_bar.max_value = max_hp
	player_hp_bar.value = current
	player_hp_label.text = "%d / %d" % [ceili(current), int(max_hp)]


func update_enemy_health(current: float, max_hp: float) -> void:
	enemy_hp_bar.max_value = max_hp
	enemy_hp_bar.value = current


func update_enemy_state(state_text: String) -> void:
	enemy_state_label.text = "IA: %s" % state_text


func update_ammo(in_mag: int, reserve: int) -> void:
	ammo_label.text = "%02d / %03d" % [in_mag, reserve]
	ammo_label.modulate = Color(1.0, 0.35, 0.25) if in_mag <= 6 else Color.WHITE


func notify_reload_started(duration: float) -> void:
	_is_reloading = true
	_reload_duration = duration
	_reload_elapsed = 0.0
	reload_bar.value = 0.0
	reload_bar.visible = true


func notify_reload_finished() -> void:
	_is_reloading = false
	reload_bar.visible = false


func notify_weapon_fired() -> void:
	_crosshair_spread_px = minf(_crosshair_spread_px + 6.5, 28.0)


func notify_hit_confirmed() -> void:
	hitmarker.visible = true
	_hitmarker_timer = 0.14


func notify_took_damage() -> void:
	_damage_flash_timer = 0.35


func _on_crouch_pressed() -> void:
	_is_crouching = not _is_crouching
	stance_label.text = "AGACHADO" if _is_crouching else "DE PIE"
	btn_crouch.modulate = Color(1.0, 0.78, 0.28) if _is_crouching else Color.WHITE
	crouch_toggled.emit(_is_crouching)


func _update_joystick_center() -> void:
	var rect := joystick_base.get_global_rect()
	_joy_center = rect.position + rect.size * 0.5
	_joy_radius = maxf(rect.size.x * 0.45, 48.0)


func _apply_joystick_drag(touch_pos: Vector2) -> void:
	var offset := touch_pos - _joy_center
	var dist := offset.length()
	if dist > _joy_radius:
		offset = offset.normalized() * _joy_radius
	joystick_knob.position = (joystick_base.size * 0.5) - (joystick_knob.size * 0.5) + offset

	var normalized := offset / _joy_radius
	move_input_changed.emit(normalized)
	sprint_changed.emit(dist >= _joy_radius * 0.86 and normalized.y < -0.55)


func _reset_joystick() -> void:
	_joy_touch_index = -1
	joystick_knob.position = (joystick_base.size * 0.5) - (joystick_knob.size * 0.5)
	move_input_changed.emit(Vector2.ZERO)
	sprint_changed.emit(false)


func _is_over_action_button(pos: Vector2) -> bool:
	for btn in [btn_fire, btn_reload, btn_crouch, btn_jump, btn_camera, btn_pause]:
		if btn.visible and btn.get_global_rect().has_point(pos):
			return true
	return false
