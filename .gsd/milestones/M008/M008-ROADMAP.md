# M008: V5.0 — "IA integrada"

**Vision:** Skeepy incorpora inteligencia artificial LOCAL — sin enviar datos a la nube —
para búsqueda semántica, resumen automático, categorización, y un grafo de conocimiento
personal. La IA es un asistente transparente que el usuario puede activar o ignorar.

**Principio:** TODO el procesamiento de IA ocurre en la máquina del usuario.
Ninguna nota sale del dispositivo salvo que el usuario lo elija explícitamente.

**Success Criteria:**
- La búsqueda semántica encuentra notas conceptualmente relacionadas aunque no compartan palabras
- Las notas largas muestran un resumen automático de 1-3 líneas
- El sistema sugiere labels para notas sin etiquetar
- Existe un graph view que muestra conexiones entre notas (backlinks + similitud semántica)
- La IA puede sugerir cómo resolver conflictos de sync basándose en el contenido

---

## Slices

- [ ] **S37: Semantic Search (sqlite-vec local)** `risk:high` `depends:[M006 completo]`
  > After this: La búsqueda en Skeepy tiene dos modos: FTS5 (exacto, ya existe) y
  > semántico (busca por significado). Para el semántico: cada nota se embeds con un
  > modelo local pequeño (ej: `nomic-embed-text` via llamafile o ONNX Runtime),
  > los embeddings se almacenan en SQLite via `sqlite-vec` extension,
  > y la búsqueda hace nearest-neighbor search sobre los embeddings.

- [ ] **S38: LLM Local — Resumen + Categorización** `risk:high` `depends:[S37]`
  > After this: Skeepy puede generar un resumen de 1-3 líneas para cualquier nota
  > (botón "Resumir" en el detail view). También sugiere labels para notas sin etiquetar.
  > Usa un LLM pequeño local (Phi-3 Mini, Gemma 2B, o llamafile) via llamafile/llama.cpp.
  > El modelo se descarga la primera vez que el usuario activa la función.
  > El usuario puede elegir no usar IA — la feature es completamente opt-in.

- [ ] **S39: Graph View + Backlinks** `risk:medium` `depends:[S37, S30]`
  > After this: Existe una vista "Grafo" en la UI que muestra las notas como nodos
  > y las conexiones entre ellas como aristas. Dos tipos de conexiones:
  > 1. Backlinks explícitos: `[[nombre de nota]]` en el texto (formato Obsidian)
  > 2. Similitud semántica: notas con embeddings cercanos (threshold configurable)
  > El grafo es interactivo: click en un nodo abre la nota, drag para reorganizar.
  > Usa una librería de visualización de grafos (ej: `vis-network` o `d3-force`).

- [ ] **S40: AI Conflict Resolution** `risk:medium` `depends:[S38, S31]`
  > After this: Cuando hay un conflicto de sync entre dos versiones de una nota,
  > la IA analiza las dos versiones y sugiere un merge automático con justificación.
  > El usuario puede aceptar la sugerencia, editarla o ignorarla y hacer el merge manual.

- [ ] **S41: Smart Sync Scheduler** `risk:low` `depends:[S38]`
  > After this: El sync no ocurre a intervalos fijos sino que la IA predice el mejor
  > momento para sincronizar basándose en el patrón de uso del usuario.
  > Si el usuario típicamente abre la app a las 9am y a las 6pm, el sync se concentra
  > en esos momentos y minimiza el uso de red y CPU en los demás horarios.

---

## Research Needed

### Embeddings Locales

**sqlite-vec** (https://github.com/asg017/sqlite-vec):
- Extensión SQLite para almacenar y buscar vectores (embeddings)
- Soporta cosine similarity, L2 distance, inner product
- Se compila como extensión `.dll` y se carga en SQLite en runtime
- Integración con `sqlx`: cargar la extensión via `LOAD EXTENSION` pragma

**Modelos de embedding locales candidatos:**
- `nomic-embed-text-v1.5`: 137M params, 768 dims, muy buena calidad/tamaño
- `all-MiniLM-L6-v2`: 22M params, 384 dims, más rápido pero menor calidad
- Ambos disponibles en formato ONNX para usar con `ort` (ONNX Runtime Rust binding)

**Flujo de indexado:**
1. Al hacer sync de una nota nueva → generar embedding en background thread
2. Almacenar en `note_embeddings(note_id, embedding BLOB)` en SQLite
3. Para búsqueda semántica: generar embedding de la query → KNN search via sqlite-vec

**Challenge:** Generar embeddings para 10k notas tarda tiempo.
Estrategia: indexar en background durante idle, priorizar notas nuevas.

### LLM Local

**llamafile** (https://github.com/Mozilla-Ocho/llamafile):
- Ejecutables portables que incluyen el modelo y el runtime (llama.cpp)
- No requieren instalación — el usuario descarga un solo archivo
- Skeepy puede descargar el llamafile la primera vez que el usuario activa la IA
- API compatible con OpenAI (POST /completion) → fácil de integrar via HTTP local

**Modelos recomendados para resumen/categorización:**
- Phi-3.5 Mini (3.8B): muy buena calidad, ~2GB, corre en CPU moderno en ~3s por resumen
- Gemma 2B: ~1.4GB, más rápido pero menor calidad
- Qwen2.5 1.5B: ~900MB, el más rápido

**Estrategia de descarga:**
- Al activar IA por primera vez: mostrar diálogo "Se descargará un modelo de IA de ~2GB.
  ¿Continuar?" con opción de elegir el modelo
- Descargar en background, mostrar progreso
- Almacenar en `%AppData%\com.skeepy.notes\models\`

### Graph View

**Librerías candidatas para Solid.js:**
- `vis-network` (https://visjs.github.io/vis-network/) — madura, muchas opciones
- `@antv/g6` — más moderna, buen soporte de React/Solid
- `d3-force` — más flexible pero requiere más código

**Consideración de performance:**
- Para > 1000 notas, el graph puede ser lento. Estrategia: solo mostrar las notas
  más conectadas (top 100) por defecto, con opción de expandir.

### Smart Sync Scheduler

Enfoque pragmático sin ML pesado:
- Registrar timestamps de apertura de la app en una tabla `usage_events`
- Calcular el promedio móvil de las últimas 4 semanas
- Programar el próximo sync para el siguiente "peak de uso" predicho
- Fallback: si no hay pattern claro, volver al intervalo fijo de Settings

---

## Nota sobre el alcance de M008

M008 representa el límite actual de lo que es técnicamente viable con hardware
de consumo en 2025-2026. Las funciones de IA requieren 4-8GB de RAM libre para
los modelos más grandes. Skeepy debe mantener su filosofía de NO ser intrusiva —
todas las features de IA son opt-in y el usuario puede usar la app perfectamente
sin ellas.

Una versión futura hipotética (M009+) podría incluir:
- Voice-to-note (whisper.cpp local)
- Síntesis de notas cross-provider ("¿qué tengo pendiente esta semana?")
- Integración con agentes AI locales (ollama, LM Studio)
- Skeepy Sync Server (self-hostable, open source)
Pero esas ideas exceden el scope de un planning útil y se dejan para cuando M008 esté completo.
