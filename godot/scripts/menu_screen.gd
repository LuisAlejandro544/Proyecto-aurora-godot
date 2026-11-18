## Pantalla principal del menú de inicio y popups de configuración/controles.
##
## Separa claramente la selección de misión (dificultad de IA, perspectiva de cámara
## y rondas) de los popups secundarios de Ajustes y Guía Táctica, manteniendo una estética
## industrial militar estructurada y legible en pantallas móviles.

class_name MenuScreen
extends Control

signal deploy_requested(difficulty: int, first_person: bool, rounds_to_win: int)
signal settings_changed(sensitivity: float, invert_y: bool)
signal sfx_volume_changed(volume_ratio: float)

@onready var btn_diff_recruit: Button = %BtnDiffRecruit
@onready var btn_diff_tactical: Button = %BtnDiffTactical
@onready var btn_diff_veteran: Button = %BtnDiffVeteran

@onready var btn_cam_tps: Button = %BtnCamTPS
@onready var btn_cam_fps: Button = %BtnCamFPS

@onready var btn_rounds_3: Button = %BtnRounds3
@onready var btn_rounds_5: Button = %BtnRounds5

@onready var btn_deploy: Button = %BtnDeploy
@onready var btn_open_settings: Button = %BtnOpenSettings
@onready var btn_open_controls: Button = %BtnOpenControls

@onready var settings_popup: PanelContainer = %SettingsPopup
@onready var controls_popup: PanelContainer = %ControlsPopup
@onready var sens_slider: HSlider = %SensSlider
@onready var sens_value_label: Label = %SensValueLabel
@onready var sfx_slider: HSlider = %SfxSlider
@onready var sfx_value_label: Label = %SfxValueLabel
@onready var invert_y_check: CheckButton = %InvertYCheck
@onready var btn_close_settings: Button = %BtnCloseSettings
@onready var btn_close_controls: Button = %BtnCloseControls

var selected_difficulty: int = 1
var selected_fps: bool = false
var selected_rounds_to_win: int = 2
var aim_sensitivity: float = 1.0
var sfx_volume: float = 1.0
var invert_y: bool = false


func _ready() -> void:
	settings_popup.visible = false
	controls_popup.visible = false

	btn_diff_recruit.pressed.connect(func(): _set_difficulty(0))
	btn_diff_tactical.pressed.connect(func(): _set_difficulty(1))
	btn_diff_veteran.pressed.connect(func(): _set_difficulty(2))

	btn_cam_tps.pressed.connect(func(): _set_camera_mode(false))
	btn_cam_fps.pressed.connect(func(): _set_camera_mode(true))

	btn_rounds_3.pressed.connect(func(): _set_rounds(2))
	btn_rounds_5.pressed.connect(func(): _set_rounds(3))

	btn_deploy.pressed.connect(func():
		deploy_requested.emit(selected_difficulty, selected_fps, selected_rounds_to_win)
	)
	btn_open_settings.pressed.connect(func():
		controls_popup.visible = false
		settings_popup.visible = true
	)
	btn_open_controls.pressed.connect(func():
		settings_popup.visible = false
		controls_popup.visible = true
	)
	btn_close_settings.pressed.connect(func(): settings_popup.visible = false)
	btn_close_controls.pressed.connect(func(): controls_popup.visible = false)

	sens_slider.value_changed.connect(_on_sens_changed)
	sfx_slider.value_changed.connect(_on_sfx_changed)
	invert_y_check.toggled.connect(_on_invert_y_toggled)

	_refresh_selection_visuals()
	_on_sens_changed(sens_slider.value)
	_on_sfx_changed(sfx_slider.value)


func _set_difficulty(level: int) -> void:
	selected_difficulty = level
	_refresh_selection_visuals()


func _set_camera_mode(fps: bool) -> void:
	selected_fps = fps
	_refresh_selection_visuals()


func _set_rounds(rounds_to_win: int) -> void:
	selected_rounds_to_win = rounds_to_win
	_refresh_selection_visuals()


func _on_sens_changed(val: float) -> void:
	aim_sensitivity = val
	sens_value_label.text = "%.1fx" % val
	settings_changed.emit(aim_sensitivity, invert_y)


func _on_sfx_changed(val: float) -> void:
	sfx_volume = val
	sfx_value_label.text = "%d%%" % roundi(val * 100.0)
	sfx_volume_changed.emit(sfx_volume)


func _on_invert_y_toggled(enabled: bool) -> void:
	invert_y = enabled
	settings_changed.emit(aim_sensitivity, invert_y)


func _refresh_selection_visuals() -> void:
	_mark_active(btn_diff_recruit, selected_difficulty == 0)
	_mark_active(btn_diff_tactical, selected_difficulty == 1)
	_mark_active(btn_diff_veteran, selected_difficulty == 2)

	_mark_active(btn_cam_tps, not selected_fps)
	_mark_active(btn_cam_fps, selected_fps)

	_mark_active(btn_rounds_3, selected_rounds_to_win == 2)
	_mark_active(btn_rounds_5, selected_rounds_to_win == 3)


func _mark_active(btn: Button, active: bool) -> void:
	btn.button_pressed = active
	btn.modulate = Color(1.0, 0.78, 0.28) if active else Color(0.72, 0.76, 0.82)
