# S18: Write Support — Local Provider

**Goal:** El usuario puede crear, editar y eliminar notas locales desde la UI.

**After this:** Ctrl+N crea una nota nueva. El NoteDetailPanel tiene un modo edición para notas locales. Guardar con Ctrl+S o blur. SQLite es el storage definitivo para notas locales.

---

## Tasks

- [x] T01 — IPC commands: note_create, note_update, note_delete
- [x] T02 — Frontend: AddNoteButton + CreateNoteModal
- [x] T03 — NoteDetailPanel: modo edición para notas locales
- [x] T04 — Keyboard shortcut Ctrl+N

## Decision

D-S18-001: SQLite es el storage permanente para notas locales desde V2.5.
El `notes.json` es solo un formato de importación — ya no se escribe de vuelta.

## Must-Haves

- Solo se puede editar notas cuyo provider_id === "local"
- Notas de Keep/Sticky Notes/Markdown: read-only con mensaje claro
- Eliminar = soft delete (is_trashed = true), no borrado físico
- Nuevo shortcut Ctrl+N global en la app
