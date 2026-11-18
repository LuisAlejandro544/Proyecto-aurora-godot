## Cargador asíncrono de modelos 3D (.glb) en tiempo de ejecución.
##
## Utiliza `ResourceLoader.load_threaded_request` con corrutinas (`await`) para cargar
## el escenario, el rifle y los efectos sin bloquear el hilo principal en Android,
## evitando referencias estáticas en los archivos `.tscn` ya que los `.glb` se descargan
## desde GitHub Releases durante la compilación.

class_name AssetLoader
extends Node

signal progress_updated(ratio: float, message: String)

const MODEL_PATHS := {
	"arena": "res://assets/models/ruined_city_arena.glb",
	"rifle": "res://assets/models/classic_assault_rifle.glb",
	"muzzle_flash": "res://assets/models/muzzle_flash_fx.glb",
	"bullet": "res://assets/models/bullet_projectile.glb",
	"casing": "res://assets/models/shell_casing_eject.glb",
}

const LABELS := {
	"arena": "Cargando ciudad destruida y coberturas...",
	"rifle": "Cargando rifle clásico 7.62...",
	"muzzle_flash": "Preparando fogonazo balístico...",
	"bullet": "Preparando proyectiles trazadores...",
	"casing": "Preparando expulsión de casquillos...",
}

var loaded_scenes: Dictionary = {}


func load_all_assets() -> bool:
	loaded_scenes.clear()
	var keys := MODEL_PATHS.keys()
	var total := float(keys.size())

	for i in range(keys.size()):
		var key: String = keys[i]
		var path: String = MODEL_PATHS[key]
		progress_updated.emit(float(i) / total, LABELS.get(key, "Cargando recursos..."))

		if not ResourceLoader.exists(path):
			push_error("No se encontró el modelo requerido: %s" % path)
			progress_updated.emit(0.0, "Falta el modelo: %s" % path)
			return false

		var err := ResourceLoader.load_threaded_request(path, "PackedScene")
		if err != OK:
			push_error("Error iniciando carga en hilo para %s (código %d)" % [path, err])
			return false

		var progress_arr := []
		while true:
			var status := ResourceLoader.load_threaded_get_status(path, progress_arr)
			if status == ResourceLoader.THREAD_LOAD_LOADED:
				var res := ResourceLoader.load_threaded_get(path)
				if res is PackedScene:
					loaded_scenes[key] = res
					break
				push_error("El recurso cargado no es PackedScene: %s" % path)
				return false
			elif status == ResourceLoader.THREAD_LOAD_FAILED or status == ResourceLoader.THREAD_LOAD_INVALID_RESOURCE:
				push_error("Falló la carga asíncrona del modelo: %s" % path)
				return false
			var sub_prog: float = progress_arr[0] if progress_arr.size() > 0 else 0.0
			progress_updated.emit((float(i) + sub_prog) / total, LABELS.get(key, "Cargando..."))
			await get_tree().process_frame

	progress_updated.emit(1.0, "Recursos 3D listos.")
	return true


func get_scene(key: String) -> PackedScene:
	return loaded_scenes.get(key, null) as PackedScene
