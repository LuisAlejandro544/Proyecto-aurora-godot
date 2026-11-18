## Renderizador vectorial modular de iconos tácticos para botones del HUD móvil.
##
## Dibuja de forma nativa y nítida en cualquier resolución DPI los iconos de:
## - "fire": bala/cartucho 7.62 inclinado con ondas de disparo.
## - "aim": retícula óptica táctica ADS para apuntar por la mira.
## - "reload": cargador curvo de rifle con flechas circulares de recarga.
## - "crouch": silueta táctica agachada / deslizando detrás de línea de cobertura.
## - "jump": impulso vertical con doble chevrón ascendente.
## - "dive": silueta en plancha (Dolphin Dive) con estelas de velocidad.
## - "grenade": granada de fragmentación táctica con contador numérico.
## - "camera": visor óptico con etiqueta 1P / 3P.
## - "pause": barras verticales de pausa.

class_name TacticalButtonIcon
extends Control

@export_enum("fire", "aim", "reload", "crouch", "jump", "dive", "grenade", "camera", "pause") var icon_kind: String = "fire":
	set(value):
		icon_kind = value
		queue_redraw()

@export var icon_color: Color = Color(0.95, 0.97, 1.0, 0.96):
	set(value):
		icon_color = value
		queue_redraw()

@export var accent_color: Color = Color(1.0, 0.78, 0.30, 1.0):
	set(value):
		accent_color = value
		queue_redraw()

@export var is_active: bool = false:
	set(value):
		if is_active != value:
			is_active = value
			queue_redraw()

@export var badge_text: String = "":
	set(value):
		if badge_text != value:
			badge_text = value
			queue_redraw()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func _draw() -> void:
	var s := size
	var center := s * 0.5
	var unit := minf(s.x, s.y)

	match icon_kind:
		"fire":
			_draw_bullet_icon(center, unit)
		"aim":
			_draw_aim_optic_icon(center, unit)
		"reload":
			_draw_magazine_reload_icon(center, unit)
		"crouch":
			_draw_crouch_icon(center, unit)
		"jump":
			_draw_jump_icon(center, unit)
		"dive":
			_draw_dive_icon(center, unit)
		"grenade":
			_draw_grenade_icon(center, unit)
		"camera":
			_draw_camera_icon(center, unit)
		"pause":
			_draw_pause_icon(center, unit)


func _draw_bullet_icon(center: Vector2, unit: float) -> void:
	var angle := -PI * 0.25
	draw_set_transform(center, angle, Vector2.ONE)

	var casing_col := Color(1.0, 0.86, 0.48, 1.0) if is_active else Color(0.98, 0.76, 0.34, 0.98)
	var tip_col := Color(1.0, 0.98, 0.92, 1.0)
	var rim_col := Color(0.86, 0.56, 0.20, 1.0)

	var rim_pts := PackedVector2Array([
		Vector2(-unit * 0.24, -unit * 0.085),
		Vector2(-unit * 0.20, -unit * 0.085),
		Vector2(-unit * 0.20, unit * 0.085),
		Vector2(-unit * 0.24, unit * 0.085),
	])
	draw_colored_polygon(rim_pts, rim_col)

	var casing_pts := PackedVector2Array([
		Vector2(-unit * 0.18, -unit * 0.078),
		Vector2(unit * 0.04, -unit * 0.072),
		Vector2(unit * 0.09, -unit * 0.050),
		Vector2(unit * 0.09, unit * 0.050),
		Vector2(unit * 0.04, unit * 0.072),
		Vector2(-unit * 0.18, unit * 0.078),
	])
	draw_colored_polygon(casing_pts, casing_col)

	var tip_pts := PackedVector2Array([
		Vector2(unit * 0.095, -unit * 0.048),
		Vector2(unit * 0.19, -unit * 0.036),
		Vector2(unit * 0.28, 0.0),
		Vector2(unit * 0.19, unit * 0.036),
		Vector2(unit * 0.095, unit * 0.048),
	])
	draw_colored_polygon(tip_pts, tip_col)

	var wave_col := Color(1.0, 0.94, 0.72, 0.85 if is_active else 0.45)
	draw_arc(Vector2(unit * 0.22, 0.0), unit * 0.13, -0.62, 0.62, 14, wave_col, 2.2, true)
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)


func _draw_aim_optic_icon(center: Vector2, unit: float) -> void:
	var ring_col := Color(0.32, 0.88, 1.0, 1.0) if is_active else icon_color
	var dot_col := accent_color if is_active else Color(1.0, 0.46, 0.24, 0.95)
	var r := unit * 0.24

	draw_arc(center, r, 0.0, TAU, 28, ring_col, 2.5, true)
	draw_circle(center, unit * 0.045, dot_col)

	var in_r := unit * 0.13
	var out_r := unit * 0.32
	draw_line(center + Vector2(0, -out_r), center + Vector2(0, -in_r), ring_col, 2.2, true)
	draw_line(center + Vector2(0, in_r), center + Vector2(0, out_r), ring_col, 2.2, true)
	draw_line(center + Vector2(-out_r, 0), center + Vector2(-in_r, 0), ring_col, 2.2, true)
	draw_line(center + Vector2(in_r, 0), center + Vector2(out_r, 0), ring_col, 2.2, true)


func _draw_magazine_reload_icon(center: Vector2, unit: float) -> void:
	var mag_col := accent_color if is_active else icon_color
	var arrow_col := Color(1.0, 0.88, 0.45, 1.0) if is_active else Color(0.46, 0.86, 1.0, 0.95)

	var mag_pts := PackedVector2Array([
		Vector2(center.x - unit * 0.09, center.y - unit * 0.16),
		Vector2(center.x + unit * 0.06, center.y - unit * 0.17),
		Vector2(center.x + unit * 0.09, center.y - unit * 0.02),
		Vector2(center.x + unit * 0.15, center.y + unit * 0.14),
		Vector2(center.x + unit * 0.01, center.y + unit * 0.18),
		Vector2(center.x - unit * 0.05, center.y + unit * 0.02),
	])
	draw_colored_polygon(mag_pts, mag_col)

	var plate_pts := PackedVector2Array([
		Vector2(center.x - unit * 0.01, center.y + unit * 0.18),
		Vector2(center.x + unit * 0.17, center.y + unit * 0.13),
		Vector2(center.x + unit * 0.18, center.y + unit * 0.17),
		Vector2(center.x + unit * 0.00, center.y + unit * 0.22),
	])
	draw_colored_polygon(plate_pts, mag_col)

	var r := unit * 0.30
	var stroke := 2.6
	draw_arc(center, r, -PI * 0.85, -PI * 0.12, 18, arrow_col, stroke, true)
	draw_arc(center, r, PI * 0.15, PI * 0.88, 18, arrow_col, stroke, true)

	_draw_arrow_tip(center + Vector2(cos(-PI * 0.12), sin(-PI * 0.12)) * r, -PI * 0.12 + PI * 0.5, unit * 0.09, arrow_col)
	_draw_arrow_tip(center + Vector2(cos(PI * 0.88), sin(PI * 0.88)) * r, PI * 0.88 + PI * 0.5, unit * 0.09, arrow_col)


func _draw_crouch_icon(center: Vector2, unit: float) -> void:
	var col := accent_color if is_active else icon_color

	draw_circle(center + Vector2(-unit * 0.02, -unit * 0.16), unit * 0.075, col)

	var body_pts := PackedVector2Array([
		Vector2(center.x - unit * 0.06, center.y - unit * 0.07),
		Vector2(center.x + unit * 0.05, center.y - unit * 0.04),
		Vector2(center.x + unit * 0.15, center.y + unit * 0.06),
		Vector2(center.x + unit * 0.07, center.y + unit * 0.19),
		Vector2(center.x - unit * 0.01, center.y + unit * 0.19),
		Vector2(center.x + unit * 0.04, center.y + unit * 0.08),
		Vector2(center.x - unit * 0.08, center.y + unit * 0.06),
		Vector2(center.x - unit * 0.16, center.y + unit * 0.19),
		Vector2(center.x - unit * 0.22, center.y + unit * 0.16),
		Vector2(center.x - unit * 0.12, center.y + unit * 0.01),
	])
	draw_colored_polygon(body_pts, col)

	draw_line(
		Vector2(center.x - unit * 0.24, center.y + unit * 0.22),
		Vector2(center.x + unit * 0.24, center.y + unit * 0.22),
		Color(col.r, col.g, col.b, 0.55),
		2.2,
		true
	)


func _draw_jump_icon(center: Vector2, unit: float) -> void:
	var col := icon_color
	var accent := Color(0.46, 0.86, 1.0, 0.95)

	var top_chev := PackedVector2Array([
		Vector2(center.x - unit * 0.17, center.y - unit * 0.03),
		Vector2(center.x, center.y - unit * 0.20),
		Vector2(center.x + unit * 0.17, center.y - unit * 0.03),
	])
	draw_polyline(top_chev, col, 3.6, true)

	var mid_chev := PackedVector2Array([
		Vector2(center.x - unit * 0.14, center.y + unit * 0.11),
		Vector2(center.x, center.y - unit * 0.04),
		Vector2(center.x + unit * 0.14, center.y + unit * 0.11),
	])
	draw_polyline(mid_chev, accent, 3.0, true)

	draw_line(
		Vector2(center.x - unit * 0.20, center.y + unit * 0.21),
		Vector2(center.x + unit * 0.20, center.y + unit * 0.21),
		Color(col.r, col.g, col.b, 0.60),
		2.4,
		true
	)


func _draw_dive_icon(center: Vector2, unit: float) -> void:
	var col := accent_color if is_active else icon_color
	var streak := Color(0.46, 0.86, 1.0, 0.92)

	# Cabeza lanzándose hacia arriba-derecha en plancha
	draw_circle(center + Vector2(unit * 0.17, -unit * 0.08), unit * 0.068, col)

	# Torso extendido diagonalmente en vuelo
	var body_pts := PackedVector2Array([
		Vector2(center.x - unit * 0.22, center.y + unit * 0.13),
		Vector2(center.x - unit * 0.04, center.y + unit * 0.02),
		Vector2(center.x + unit * 0.11, center.y - unit * 0.05),
		Vector2(center.x + unit * 0.14, center.y + unit * 0.03),
		Vector2(center.x - unit * 0.02, center.y + unit * 0.10),
		Vector2(center.x - unit * 0.19, center.y + unit * 0.19),
	])
	draw_colored_polygon(body_pts, col)

	# Estelas de velocidad detrás del operador
	draw_line(
		center + Vector2(-unit * 0.26, -unit * 0.03),
		center + Vector2(-unit * 0.10, -unit * 0.07),
		streak,
		2.3,
		true
	)
	draw_line(
		center + Vector2(-unit * 0.28, unit * 0.06),
		center + Vector2(-unit * 0.14, unit * 0.03),
		streak,
		2.0,
		true
	)


func _draw_grenade_icon(center: Vector2, unit: float) -> void:
	var has_ammo := badge_text != "0"
	var body_col := Color(0.62, 0.80, 0.44, 1.0) if has_ammo else Color(0.45, 0.48, 0.52, 0.55)
	var steel_col := icon_color if has_ammo else Color(0.55, 0.58, 0.62, 0.55)

	var body_c := center + Vector2(-unit * 0.03, unit * 0.04)
	# Cuerpo ovoide de granada
	var body_pts := PackedVector2Array([
		body_c + Vector2(-unit * 0.10, -unit * 0.14),
		body_c + Vector2(unit * 0.10, -unit * 0.14),
		body_c + Vector2(unit * 0.14, -unit * 0.04),
		body_c + Vector2(unit * 0.14, unit * 0.08),
		body_c + Vector2(unit * 0.08, unit * 0.17),
		body_c + Vector2(-unit * 0.08, unit * 0.17),
		body_c + Vector2(-unit * 0.14, unit * 0.08),
		body_c + Vector2(-unit * 0.14, -unit * 0.04),
	])
	draw_colored_polygon(body_pts, body_col)

	# Líneas de fragmentación
	draw_line(body_c + Vector2(-unit * 0.13, -unit * 0.01), body_c + Vector2(unit * 0.13, -unit * 0.01), Color(0.10, 0.14, 0.12, 0.8), 1.8, true)
	draw_line(body_c + Vector2(-unit * 0.12, unit * 0.08), body_c + Vector2(unit * 0.12, unit * 0.08), Color(0.10, 0.14, 0.12, 0.8), 1.8, true)

	# Espoleta superior y palanca de seguridad
	draw_rect(Rect2(body_c.x - unit * 0.06, body_c.y - unit * 0.23, unit * 0.12, unit * 0.09), steel_col, true)
	var lever_pts := PackedVector2Array([
		body_c + Vector2(unit * 0.04, -unit * 0.22),
		body_c + Vector2(unit * 0.18, -unit * 0.17),
		body_c + Vector2(unit * 0.22, unit * 0.03),
		body_c + Vector2(unit * 0.17, unit * 0.03),
		body_c + Vector2(unit * 0.14, -unit * 0.14),
		body_c + Vector2(unit * 0.04, -unit * 0.18),
	])
	draw_colored_polygon(lever_pts, steel_col)
	draw_arc(body_c + Vector2(-unit * 0.11, -unit * 0.19), unit * 0.055, 0.0, TAU, 14, accent_color if has_ammo else steel_col, 2.0, true)

	if not badge_text.is_empty():
		var font := ThemeDB.fallback_font
		var font_size := int(clampf(unit * 0.25, 11.0, 16.0))
		var badge_col := accent_color if has_ammo else Color(0.88, 0.34, 0.28, 0.9)
		draw_string(
			font,
			Vector2(center.x + unit * 0.10, center.y + unit * 0.31),
			badge_text,
			HORIZONTAL_ALIGNMENT_CENTER,
			unit * 0.28,
			font_size,
			badge_col
		)


func _draw_camera_icon(center: Vector2, unit: float) -> void:
	var col := icon_color
	var eye_center := center + Vector2(0.0, -unit * 0.09)

	var top_arc := PackedVector2Array([
		eye_center + Vector2(-unit * 0.24, 0.0),
		eye_center + Vector2(-unit * 0.10, -unit * 0.11),
		eye_center + Vector2(0.0, -unit * 0.13),
		eye_center + Vector2(unit * 0.10, -unit * 0.11),
		eye_center + Vector2(unit * 0.24, 0.0),
	])
	var bot_arc := PackedVector2Array([
		eye_center + Vector2(-unit * 0.24, 0.0),
		eye_center + Vector2(-unit * 0.10, unit * 0.11),
		eye_center + Vector2(0.0, unit * 0.13),
		eye_center + Vector2(unit * 0.10, unit * 0.11),
		eye_center + Vector2(unit * 0.24, 0.0),
	])
	draw_polyline(top_arc, col, 2.2, true)
	draw_polyline(bot_arc, col, 2.2, true)
	draw_circle(eye_center, unit * 0.065, accent_color if is_active else Color(0.46, 0.86, 1.0, 1.0))

	if not badge_text.is_empty():
		var font := ThemeDB.fallback_font
		var font_size := int(clampf(unit * 0.24, 10.0, 15.0))
		var text_pos := Vector2(center.x - unit * 0.32, center.y + unit * 0.28)
		draw_string(
			font,
			text_pos,
			badge_text,
			HORIZONTAL_ALIGNMENT_CENTER,
			unit * 0.64,
			font_size,
			col
		)


func _draw_pause_icon(center: Vector2, unit: float) -> void:
	var bar_w := unit * 0.10
	var bar_h := unit * 0.36
	var gap := unit * 0.075
	draw_rect(
		Rect2(center.x - gap - bar_w, center.y - bar_h * 0.5, bar_w, bar_h),
		icon_color,
		true
	)
	draw_rect(
		Rect2(center.x + gap, center.y - bar_h * 0.5, bar_w, bar_h),
		icon_color,
		true
	)


func _draw_arrow_tip(tip_pos: Vector2, heading_rad: float, size_px: float, color: Color) -> void:
	var fwd := Vector2(cos(heading_rad), sin(heading_rad))
	var right := Vector2(-fwd.y, fwd.x)
	var pts := PackedVector2Array([
		tip_pos + fwd * (size_px * 0.65),
		tip_pos - fwd * (size_px * 0.45) + right * (size_px * 0.55),
		tip_pos - fwd * (size_px * 0.45) - right * (size_px * 0.55),
	])
	draw_colored_polygon(pts, color)
