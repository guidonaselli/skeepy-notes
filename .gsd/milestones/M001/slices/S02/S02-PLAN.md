# S02: Storage Layer

**Goal:** Implementar el crate `skeepy-storage` con SQLite (WAL + FTS5) que satisface los traits de repositorio definidos en `skeepy-core`.
**Demo:** `cargo test -p skeepy-storage` pasa. Insertar/buscar/actualizar notas funciona en una DB real.

## Must-Haves

- `SqliteDatabase::open(path)` inicializa la DB, activa WAL mode, y corre migrations
- Migrations idempotentes: pueden correr múltiples veces sin error
- FTS5 indexa título + texto, se mantiene sincronizado via triggers
- `NoteRepository` completamente implementado (find_all, find_by_id, search_fts, upsert, update_layout, etc.)
- `SettingsRepository` implementado (get/set de JSON por key)
- Tests de integración contra una DB SQLite real (in-memory o tempfile)

## Tasks

- [ ] **T01: Database setup** (db.rs, migrations, WAL+FTS5)
- [ ] **T02: NoteRepository** (note_repository.rs — conversión Row↔Note, todos los métodos)
- [ ] **T03: SettingsRepository** (settings_repository.rs)
- [ ] **T04: Integration tests**

## Files Likely Touched

- `src-tauri/crates/storage/src/lib.rs`
- `src-tauri/crates/storage/src/db.rs`
- `src-tauri/crates/storage/migrations/001_initial.sql`
- `src-tauri/crates/storage/migrations/002_fts5.sql`
- `src-tauri/crates/storage/src/repositories/note_repository.rs`
- `src-tauri/crates/storage/src/repositories/settings_repository.rs`
- `src-tauri/crates/storage/src/repositories/mod.rs`
