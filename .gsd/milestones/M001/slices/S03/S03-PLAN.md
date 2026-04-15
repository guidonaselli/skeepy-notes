# S03: Local JSON Provider

**Goal:** Implementar `skeepy-provider-local` que lee notas desde un archivo JSON en disco y satisface el trait `NoteProvider`.
**Demo:** `cargo test -p skeepy-provider-local` pasa. Se puede configurar una ruta a un archivo JSON de notas y el provider las devuelve como `Vec<RemoteNote>`.

## Must-Haves

- `LocalProvider` implementa `NoteProvider` trait completamente
- Lee notas desde un archivo JSON configurable
- Si el archivo no existe → devuelve `Vec::new()` (no error)
- Si el archivo tiene formato inválido → devuelve `ProviderError::Api` con mensaje claro
- `capabilities()` reporta `can_read: true, can_write: false, stability: Stable`
- `is_authenticated()` devuelve `true` siempre (no requiere auth)
- `authenticate()` no hace nada (no hay credenciales que gestionar)
- Tests: file no existe, file válido, file inválido

## Tasks

- [ ] **T01: LocalProvider** (provider.rs)
  Struct, impl NoteProvider, file reading, JSON deserialization.
- [ ] **T02: Data format** (format.rs)
  LocalNote struct para el formato del archivo JSON. Mapping a RemoteNote.
- [ ] **T03: Tests**

## Files Likely Touched

- `src-tauri/crates/providers/local/src/lib.rs`
- `src-tauri/crates/providers/local/src/provider.rs`
- `src-tauri/crates/providers/local/src/format.rs`
