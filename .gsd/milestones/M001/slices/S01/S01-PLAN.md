# S01: Core Domain

**Goal:** Definir el modelo de dominio completo en el crate `skeepy-core` — sin dependencias de I/O. Todo testeable en aislamiento.
**Demo:** `cargo check` y `cargo test -p skeepy-core` pasan sin errores.

## Must-Haves

- `Note` struct con todos sus campos y métodos básicos
- `NoteContent`, `NoteColor`, `NoteLayout`, `SyncState`, `Label` implementados
- `NoteProvider` trait con async (via `async-trait`) y defaults para operaciones de escritura
- `NoteRepository` trait (port para el storage) en el dominio
- `NoteService` con lógica de merge (pull-only, sin I/O directo)
- `SyncOrchestrator` con BackoffConfig y lógica de scheduling
- `AppSettings` con defaults correctos
- Tests unitarios para la lógica de negocio no trivial

## Tasks

- [ ] **T01: Cargo workspace + core crate scaffold**
  Workspace root Cargo.toml, crate `skeepy-core` con dependencias, stubs para crates restantes.

- [ ] **T02: Domain types** (`note.rs`, `provider.rs`, `repository.rs`, `settings.rs`, `error.rs`)
  Todas las structs, enums y traits del dominio. Sin implementaciones de I/O.

- [ ] **T03: NoteService + SyncOrchestrator**
  Lógica de merge y orchestración. `NoteService::merge_remote()` con los 4 casos de sync state.
  `SyncOrchestrator` con BackoffConfig, cooldown check, provider iteration.

- [ ] **T04: Unit tests**
  Tests para `NoteContent::text_preview`, `SyncState`, `BackoffConfig::delay_for_attempt`,
  `NoteService::merge_remote` con un InMemoryRepository mock.

## Files Likely Touched

- `Cargo.toml` (workspace root)
- `src-tauri/crates/core/`
- `src-tauri/crates/storage/` (stub)
- `src-tauri/crates/providers/local/` (stub)
- `src-tauri/crates/providers/keep/` (stub)
- `src-tauri/crates/os_integration/` (stub)
