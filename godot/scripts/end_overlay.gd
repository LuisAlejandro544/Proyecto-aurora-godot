## Pantalla superpuesta de Pausa y Fin de Ronda / Partida.
##
## Separa el menú de pausa (reanudar, sensibilidad rápida, cambio de cámara, volver al menú)
## y la pantalla de resultado con estadísticas de impacto y botones de siguiente ronda o revancha.

class_name EndOverlay
extends Control

signal resume_pressed()
signal next_round_pressed()
signal rematch_pressed()
signal return_menu_pressed()
signal quick_sensitivity_changed(val: float)
signal quick_camera_toggled()

@onready var title_label: Label = %OverlayTitle
@onready var subtitle_label: Label = %OverlaySubtitle
@onready var stats_label: Label = %OverlayStats
@onready var quick_sens_box: Control = %QuickSensBox
@onready var quick_sens_slider: HSlider = %QuickSensSlider
@onready var btn_primary: Button = %BtnOverlayPrimary
@onready var btn_secondary: Button = %BtnOverlaySecondary
@onready var btn_menu: Button = %BtnOverlayMenu

enum Mode { PAUSE, ROUND_END, MATCH_END }
var _mode: Mode = Mode.PAUSE


func _ready() -> void:
	visible = false
	btn_primary.pressed.connect(_on_primary_pressed)
	btn_secondary.pressed.connect(_on_secondary_pressed)
	btn_menu.pressed.connect(func(): return_menu_pressed.emit())
	quick_sens_slider.value_changed.connect(func(v: float): quick_sensitivity_changed.emit(v))


func show_pause(p_score: int, ai_score: int, sens: float) -> void:
	_mode = Mode.PAUSE
	visible = true
	title_label.text = "COMBATE EN PAUSA"
	title_label.modulate = Color(0.95, 0.85, 0.45)
	subtitle_label.text = "Marcador actual: TÚ %d  -  %d IA" % [p_score, ai_score]
	stats_label.text = "Ajusta la sensibilidad táctil o cambia de cámara sin perder tu ronda."
	quick_sens_box.visible = true
	quick_sens_slider.set_value_no_signal(sens)
	btn_primary.text = "REANUDAR COMBATE"
	btn_secondary.visible = true
	btn_secondary.text = "CAMBIAR CÁMARA"
	btn_menu.text = "VOLVER AL MENÚ"


func show_round_result(player_won: bool, p_score: int, ai_score: int, hits: int) -> void:
	_mode = Mode.ROUND_END
	visible = true
	quick_sens_box.visible = false
	title_label.text = "RONDA GANADA" if player_won else "RONDA PERDIDA"
	title_label.modulate = Color(0.28, 0.88, 0.56) if player_won else Color(0.96, 0.34, 0.28)
	subtitle_label.text = "Marcador global: TÚ %d  -  %d IA" % [p_score, ai_score]
	stats_label.text = "Impactos acertados esta ronda: %d" % hits
	btn_primary.text = "SIGUIENTE RONDA"
	btn_secondary.visible = false
	btn_menu.text = "SALIR AL MENÚ"


func show_match_result(player_won: bool, p_score: int, ai_score: int, total_hits: int) -> void:
	_mode = Mode.MATCH_END
	visible = true
	quick_sens_box.visible = false
	title_label.text = "VICTORIA EN ZONA CERO" if player_won else "DERROTA TÁCTICA"
	title_label.modulate = Color(0.32, 0.92, 0.62) if player_won else Color(0.96, 0.32, 0.26)
	subtitle_label.text = "Resultado final: TÚ %d  -  %d IA" % [p_score, ai_score]
	stats_label.text = "Impactos totales conectados con AK-7.62: %d" % total_hits
	btn_primary.text = "JUGAR REVANCHA"
	btn_secondary.visible = false
	btn_menu.text = "VOLVER AL MENÚ"


func _on_primary_pressed() -> void:
	match _mode:
		Mode.PAUSE:
			resume_pressed.emit()
		Mode.ROUND_END:
			next_round_pressed.emit()
		Mode.MATCH_END:
			rematch_pressed.emit()


func _on_secondary_pressed() -> void:
	if _mode == Mode.PAUSE:
		quick_camera_toggled.emit()
