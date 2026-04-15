# M003: V2.0 — "Más providers, lectura enriquecida"

**Vision:** Skeepy deja de ser un visor de dos providers y se convierte en el agregador
de notas Windows por excelencia. Sticky Notes y Markdown se suman sin necesitar configuración
técnica. El usuario puede ver el contenido completo de cualquier nota y gestionar sus providers
desde la UI.

**Success Criteria:**
- Windows Sticky Notes importa automáticamente al instalarse (sin configuración)
- Una carpeta de archivos Markdown se puede agregar como provider desde la UI
- Existe un panel de detalle de nota (click en card → nota expandida con scroll)
- El usuario puede agregar, quitar y reordenar providers desde Settings
- Los errores de sync se muestran claramente en la UI con acción sugerida

---

## Slices

- [ ] **S13: Windows Sticky Notes Provider** `risk:medium` `depends:[S09]`
  > After this: Las notas de Windows Sticky Notes aparecen en Skeepy automáticamente.
  > El provider lee la DB local de Sticky Notes (`plum.sqlite`) sin ninguna configuración
  > del usuario. Si el archivo no existe, el provider simplemente no devuelve notas.

- [ ] **S14: Markdown Folder Provider** `risk:low` `depends:[S09]`
  > After this: El usuario puede agregar una carpeta como "provider Markdown" desde la UI.
  > Cada archivo `.md` en esa carpeta (no recursivo por defecto) se convierte en una nota.
  > El título es el nombre del archivo, el cuerpo es el contenido. Watch de cambios con
  > `notify` crate para sync automático al guardar.

- [ ] **S15: Note Detail View** `risk:low` `depends:[S05]`
  > After this: Hacer click en una NoteCard abre un panel lateral (o modal) con el contenido
  > completo de la nota: título, body con scroll, checklist interactivo (toggle de items),
  > labels, color, provider de origen, timestamps. En V2 es read-only.

- [ ] **S16: Provider Manager UI** `risk:medium` `depends:[S10]`
  > After this: En Settings existe una sección "Providers" donde el usuario puede ver todos
  > los providers disponibles (local, keep, sticky-notes, markdown), activar/desactivar cada uno,
  > configurar los que necesitan configuración (ej: path para markdown), y ver el estado de sync.

- [ ] **S17: Sync Robustez + Error Recovery UI** `risk:low` `depends:[S10]`
  > After this: Cuando un provider falla, la UI muestra un banner no-intrusivo con el error
  > y las opciones "Reintentar" / "Desactivar provider". El sync engine respeta el backoff
  > exponencial. Sleep/Resume del PC cancela el sync en curso y re-trigerea al despertar.

---

## Research Needed

### Windows Sticky Notes DB
- Path: `%LocalAppData%\Packages\Microsoft.MicrosoftStickyNotes_8wekyb3d8bbwe\LocalState\plum.sqlite`
- Schema a investigar: tablas `Note`, `NoteMedia`, `StickyNotesGroup`
- El archivo puede estar bloqueado por el proceso de Sticky Notes — usar WAL mode o copiar a temp antes de leer
- Contenido: texto en formato OOXML o texto plano (a verificar)

### Markdown Folder Watcher
- Crate `notify` (https://docs.rs/notify) para inotify/kqueue/ReadDirectoryChangesW
- El watcher debe ser debounced (evitar re-syncs por cada keypress al guardar)
- Parsear frontmatter YAML si existe (`---` al inicio) para extraer título, tags, color
- Ignorar archivos que empiecen con `.` o `_`

### Tauri Plugin Shell (para el Provider Manager)
- El markdown provider necesita un "file picker" para seleccionar la carpeta
- `tauri-plugin-dialog` → `open({ directory: true })` para seleccionar la carpeta de markdown

---

## Dependency Map

```
S13 (Sticky Notes) → independiente de S14, S15, S16
S14 (Markdown)     → independiente de S13
S15 (Detail View)  → independiente de S13, S14, S16
S16 (Provider Mgr) → depends S13, S14 (para mostrar su configuración)
S17 (Robustez)     → depends S10 (Provider Status del M002)
```

Orden recomendado: S15 → S13 → S14 → S16 → S17
(S15 es independiente y agrega valor inmediato; S16 necesita a S13 y S14 para ser útil)
