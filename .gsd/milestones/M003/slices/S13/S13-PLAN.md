# S13: Windows Sticky Notes Provider

**Goal:** Importar las notas de Windows Sticky Notes automáticamente, sin configuración del usuario.

**After this:** Las notas de Windows Sticky Notes aparecen en Skeepy al arrancar la app, igual que las notas del JSON local. Sin OAuth, sin configuración.

---

## Tasks

- [x] T01 — Crear `sticky_notes_provider` crate en `src-tauri/crates/`
- [x] T02 — Implementar lectura de `plum.sqlite` (path detection + WAL copy)
- [x] T03 — Mapear schema de `plum.sqlite` a `Note` del dominio
- [x] T04 — Registrar `StickyNotesProvider` en `AppState` al arrancar
- [x] T05 — Verificar que las notas aparecen en el grid (must-haves)

## Must-Haves

- Si `plum.sqlite` no existe → provider devuelve `[]` sin error
- Si Sticky Notes está abierto (DB bloqueada) → copiar a temp y leer desde ahí
- `provider_id` = `"windows_sticky_notes"` (constante)
- El contenido OOXML se stripea a texto plano
- No requiere configuración del usuario

## Schema esperado de plum.sqlite

Tablas relevantes:
- `Note` — id (TEXT), createdAt, deletedAt, updatedAt, parentId
- `NoteMedia` — id, noteId (FK a Note.id), type, data (texto/OOXML), mime (image/*)
- `StickyNotesGroup` — opcional, para grupos

El contenido de texto está en `NoteMedia` con `mime = 'text/plain'` o como OOXML en `type = 0`.
