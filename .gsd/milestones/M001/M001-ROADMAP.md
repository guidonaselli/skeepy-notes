# M001: V1 — "Funciona, vive en tu PC, no la molesta"

**Vision:** Una app residente de escritorio para Windows que muestra notas de múltiples orígenes (local + Google Keep), vive en el system tray, consume menos de 50MB de RAM en idle, arranca en menos de 1 segundo, funciona offline, y recuerda la posición y tamaño de cada nota. Sin editor inline, sin write a providers — solo lectura, visualización y búsqueda impecables.

**Success Criteria:**
- App arranca en < 1s y aparece en el system tray de Windows
- Notas del provider local se muestran como cards en la UI
- Notas de Google Keep se importan via OAuth2 (read-only) y se muestran junto a las locales
- Búsqueda full-text responde en < 50ms para 1000 notas
- Layout (posición, tamaño, color) persiste entre reinicios de la app
- App consume < 50MB RAM y < 0.5% CPU en idle
- Cierre de ventana minimiza al tray (proceso sigue corriendo)
- Autostart con Windows habilitado por default
- Tokens de OAuth almacenados en Windows Credential Manager (nunca en disco plano)
- Instalador NSIS funcional con uninstall limpio

---

## Slices

- [x] **S01: Core Domain** `risk:low` `depends:[]`
  > After this: El modelo de dominio está definido y compilando — Note, NoteProvider trait, SyncState, NoteLayout, AppSettings. Tests unitarios del dominio pasan.

- [x] **S02: Storage Layer** `risk:medium` `depends:[S01]`
  > After this: SQLite con WAL + FTS5 inicializa, migrations corren, se pueden insertar/leer/buscar notas. Tests de integración con DB real pasan.

- [x] **S03: Local JSON Provider** `risk:low` `depends:[S01,S02]`
  > After this: Se puede crear un archivo JSON de notas locales y la app las carga, persiste y muestra en SQLite. El sync engine básico (SyncOrchestrator) corre el primer ciclo.

- [x] **S04: Tauri Shell + Windows Integration** `risk:medium` `depends:[S01]`
  > After this: App Tauri arranca, muestra ventana principal, tiene tray icon funcional, X minimiza al tray, "Salir" en tray cierra el proceso, autostart con Windows funciona.

- [x] **S05: Solid.js UI — Core** `risk:low` `depends:[S04,S02]`
  > After this: La UI muestra NoteCards con posición y tamaño persistente, barra de búsqueda con FTS5, filtro por provider, panel de Settings básico. Layout persiste entre reinicios.

- [x] **S06: Google Keep Provider** `risk:high` `depends:[S01,S02,S03]`
  > After this: El usuario puede conectar su cuenta de Google (OAuth2, keep.readonly), las notas de Keep se importan a SQLite, se muestran mezcladas con las locales, sync periódico funciona.

- [x] **S07: Polish + QA** `risk:low` `depends:[S05,S06]`
  > After this: App pasa todos los criterios de aceptación medibles, backoff/retry funciona correctamente ante errores de red, comportamiento offline verificado, memoria estable en 8h de uso.

- [ ] **S08: NSIS Installer + Release** `risk:low` `depends:[S07]`
  > After this: `cargo tauri build` produce un .exe instalador NSIS, instalación limpia, shortcut en inicio, uninstall limpio. Release pipeline en GitHub Actions genera el binario.

---

## Boundary Map

### S01 → todos los demás
Produces (crate `core`):
- `note.rs` → `Note`, `NoteContent`, `ChecklistItem`, `NoteColor`, `NoteLayout`, `Label`, `SyncState`, `NoteId`, `ProviderId`
- `provider.rs` → trait `NoteProvider`, `RemoteNote`, `ProviderCapabilities`, `ProviderStatus`, `ProviderStability`, `ProviderError`, `CreateNoteRequest`
- `services/note_service.rs` → `NoteService` (CRUD + merge logic)
- `services/sync_orchestrator.rs` → `SyncOrchestrator`, `SyncTrigger`, `BackoffConfig`

Consumes: nothing (leaf node)

### S02 → S03, S05, S06, S07
Produces (crate `storage`):
- `db.rs` → `Database::connect()`, `Database::run_migrations()`
- `repositories/note_repository.rs` → `NoteRepository::insert()`, `update()`, `find_by_id()`, `find_all()`, `search_fts()`, `find_by_provider()`
- `repositories/settings_repository.rs` → `SettingsRepository::get()`, `set()`
- SQLite schema con tablas: `notes`, `note_layouts`, `labels`, `note_labels`, `notes_fts`, `provider_sync_state`, `settings`

Consumes from S01:
- `core::Note`, `core::NoteLayout`, `core::SyncState`

### S03 → S05, S06
Produces (crate `providers/local`):
- `LocalProvider` que implementa `NoteProvider` trait
- Leer notas desde archivo JSON configurado por el usuario
- `SyncOrchestrator` básico funcional con un provider registrado

Consumes from S01:
- `NoteProvider` trait, `RemoteNote`, `ProviderCapabilities`
Consumes from S02:
- `NoteRepository::insert()`, `update()`

### S04 → S05
Produces (Tauri app shell):
- App binario Tauri con ventana principal y tray icon
- IPC commands registrados: `notes_get_all`, `notes_search`, `sync_trigger`, `settings_get`, `settings_set`
- Tray menu: "Mostrar", "Sincronizar ahora", "Configuración", "Salir"
- `tauri-plugin-autostart` configurado
- Evento `sync://progress` emitido hacia frontend

Consumes from S01:
- `AppSettings` struct para configuración inicial

### S05 → S06, S07
Produces (Solid.js frontend):
- Componentes: `NoteCard`, `NoteGrid`, `SearchBar`, `ProviderBadge`, `Settings`
- Stores: `notes.store.ts` (Map<NoteId, Note>), `sync.store.ts` (estado por provider)
- `tauri.service.ts` — wrapper de `invoke()` tipado
- Layout persistente: drag-to-move, resize, save on mouse-up via IPC

Consumes from S04:
- IPC commands y eventos de Tauri

### S06 → S07
Produces (crate `providers/keep`):
- `KeepProvider` que implementa `NoteProvider` trait
- OAuth2 flow: `tauri-plugin-oauth` + `oauth2` crate, scope `keep.readonly`
- Token CRUD via `keyring` crate (DPAPI)
- `notes.googleapis.com/v1` client: `list_notes()`, `get_note()`
- Mapping de KeepNote → RemoteNote (colores, labels, checklist, timestamps)
- Rate limiter via `governor` crate

Consumes from S01:
- `NoteProvider` trait, `RemoteNote`, `ProviderCapabilities`, `ProviderStability::Experimental`
Consumes from S02:
- `NoteRepository`, `provider_sync_state` table

### S07 → S08
Produces:
- Todos los criterios de aceptación verificados y documentados
- Bug fixes de S01-S06
- `S07-UAT.md` con checklist de pruebas manuales

### S08 → (final)
Produces:
- `.exe` instalador NSIS via `cargo tauri build`
- GitHub Actions workflow para release
- `README.md` con instrucciones de instalación y setup de Keep
