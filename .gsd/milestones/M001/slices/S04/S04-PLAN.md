# S04: Tauri Shell + Windows Integration

**Goal:** Crear el binario Tauri que conecta el backend Rust con el frontend. Sistema tray funcional, X minimiza al tray, autostart con Windows, IPC commands registrados.
**Demo:** `cargo tauri dev` arranca la app, aparece en el system tray, el menú tiene "Mostrar / Salir", cerrar la ventana la oculta (no mata el proceso), "Abrir al iniciar Windows" funciona via registro HKCU.

## Must-Haves

- Binario Tauri 2.x en `src-tauri/` con `main.rs` y `lib.rs`
- `src-tauri/Cargo.toml` correctamente configurado (skeepy-app)
- `src-tauri/tauri.conf.json` con identifier, ventana inicial, bundle NSIS
- Tray icon con menú: "Mostrar Skeepy", "Sincronizar ahora", "---", "Salir"
- `on_window_event` CloseRequested → `hide()` (no cerrar)
- `tauri-plugin-autostart` configurado, habilitado por default en primer arranque
- IPC commands registrados: `notes_get_all`, `notes_search`, `sync_trigger`, `settings_get`, `settings_set`
- `AppState` con `Arc<Database>`, `Arc<dyn NoteRepository>`, lista de providers
- Evento `sync://progress` emitido desde backend hacia frontend
- Workspace Cargo.toml actualizado con `src-tauri` como miembro

## Out of Scope

- Frontend real (S05 lo hace) — solo necesitamos una página HTML placeholder
- Keep provider (S06)
- Tests de integración end-to-end con UI

## Tasks

- [ ] **T01: Workspace + Cargo setup**
  - Actualizar `Cargo.toml` raíz para incluir `src-tauri` como miembro del workspace
  - Crear `src-tauri/Cargo.toml` con dependencias Tauri 2.x
  - Crear `src-tauri/src/main.rs` y `src-tauri/src/lib.rs` stub

- [ ] **T02: AppState + app builder**
  - `src-tauri/src/state.rs` — `AppState` struct con Database, NoteRepository, providers
  - `src-tauri/src/lib.rs` — `run()` con `tauri::Builder`, plugins, commands, tray

- [ ] **T03: IPC Commands**
  - `src-tauri/src/commands/notes.rs` — `notes_get_all`, `notes_search`
  - `src-tauri/src/commands/sync.rs` — `sync_trigger`
  - `src-tauri/src/commands/settings.rs` — `settings_get`, `settings_set`

- [ ] **T04: Tray + window behavior**
  - Tray icon y menú
  - `CloseRequested` → hide window
  - `sync://progress` event emission

- [ ] **T05: Autostart**
  - `tauri-plugin-autostart` integrado
  - Habilitado en primer arranque si no existe setting previo

- [ ] **T06: tauri.conf.json**
  - Identifier, ventana, bundle NSIS config, icons

- [ ] **T07: Placeholder frontend**
  - `src/index.html` mínimo para que `cargo tauri dev` compile sin frontend real

## Files Likely Touched

- `Cargo.toml` (raíz) — agregar `src-tauri` a workspace members
- `src-tauri/Cargo.toml`
- `src-tauri/src/main.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/notes.rs`
- `src-tauri/src/commands/sync.rs`
- `src-tauri/src/commands/settings.rs`
- `src-tauri/tauri.conf.json`
- `src/index.html`
