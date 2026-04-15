# M001: V1 — "Funciona, vive en tu PC, no la molesta" — Context

**Gathered:** 2026-04-12
**Status:** Ready for planning

---

## Vision del Producto

Skeepy es un **agregador/visor de notas local con arquitectura por proveedores** — NO un cliente de Google Keep. Es una app residente de escritorio para Windows que:

- Vive 24/7 sin que el usuario la sienta (< 50MB RAM idle, ~0% CPU idle)
- Funciona completamente offline — storage local es la source of truth
- Soporta múltiples orígenes de notas via providers desacoplados
- Muestra notas en una interfaz estilo sticky notes con layout persistente
- Arranca con Windows, vive en el tray, no molesta

**La app debe funcionar perfectamente aunque Google Keep desaparezca mañana.**

---

## Stack Decidido

| Layer | Tecnología | Justificación |
|---|---|---|
| Backend/Core | Rust | Sin GC, sin runtime, 0% CPU idle, ownership elimina data races |
| Desktop Shell | Tauri 2.x | WebView2 nativo Win10/11, tray first-class, plugins oficiales |
| UI | Solid.js | DOM directo, sin Virtual DOM, bundle mínimo, reactividad granular |
| Storage | SQLite + FTS5 (sqlx) | Zero-config, WAL mode, FTS5 nativa para search |
| Credentials | keyring crate (DPAPI) | Windows Credential Manager, cifrado OS-level |
| Async | Tokio | Estándar Rust async, sin blocking threads |
| Packaging | cargo-tauri + NSIS | Installer limpio, shortcut en inicio, uninstall limpio |

---

## Estructura del Repo (target)

```
skeepy/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                 # Entry point + Tauri builder
│   │   ├── commands/               # IPC handlers (thin)
│   │   │   ├── notes.rs
│   │   │   ├── sync.rs
│   │   │   └── settings.rs
│   │   └── lib.rs
│   ├── crates/
│   │   ├── core/                   # Domain logic — NO I/O, 100% testeable
│   │   │   └── src/
│   │   │       ├── note.rs         # Note entity, NoteContent, SyncState, NoteLayout
│   │   │       ├── provider.rs     # NoteProvider trait, ProviderCapabilities, ProviderStatus
│   │   │       └── services/
│   │   │           ├── note_service.rs
│   │   │           └── sync_orchestrator.rs
│   │   ├── storage/                # SQLite + FTS5
│   │   │   └── src/
│   │   │       ├── db.rs
│   │   │       ├── migrations/
│   │   │       └── repositories/
│   │   │           ├── note_repository.rs
│   │   │           └── settings_repository.rs
│   │   ├── providers/
│   │   │   ├── local/              # Local JSON provider (V1)
│   │   │   ├── keep/               # Google Keep provider (V1, read-only)
│   │   │   └── markdown/           # Markdown/TXT provider (V2)
│   │   └── os_integration/         # DPAPI, autostart, tray helpers
│   └── Cargo.toml
├── src/                            # Solid.js frontend
│   ├── components/
│   │   ├── NoteCard/
│   │   ├── SearchBar/
│   │   ├── ProviderBadge/
│   │   └── Settings/
│   ├── stores/
│   │   ├── notes.store.ts
│   │   └── sync.store.ts
│   └── services/
│       └── tauri.service.ts        # IPC bridge wrapper
├── tests/integration/
├── docs/
├── .gsd/                           # Este directorio
├── Cargo.toml                      # Workspace root
├── package.json
└── tauri.conf.json
```

---

## Domain Model — Contratos Clave

### Entidad Note (Rust)

```rust
pub struct Note {
    pub id: NoteId,                  // UUID interno estable
    pub source_id: String,           // ID nativo del provider (Keep note ID, file path, etc.)
    pub provider_id: ProviderId,     // "local", "keep", etc.
    pub title: Option<String>,
    pub content: NoteContent,        // Text(String) | Checklist(Vec<ChecklistItem>)
    pub labels: Vec<Label>,
    pub color: NoteColor,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub is_trashed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
    pub sync_state: SyncState,
    pub layout: NoteLayout,
}
```

### NoteProvider Trait (Rust)

```rust
#[async_trait]
pub trait NoteProvider: Send + Sync {
    fn id(&self) -> &ProviderId;
    fn name(&self) -> &str;
    fn status(&self) -> ProviderStatus;
    fn capabilities(&self) -> ProviderCapabilities;
    async fn authenticate(&mut self) -> Result<(), ProviderError>;
    async fn is_authenticated(&self) -> bool;
    async fn fetch_notes(&self, since: Option<DateTime<Utc>>) -> Result<Vec<RemoteNote>, ProviderError>;
    // Write methods default to ProviderError::NotSupported (V1 no los implementa)
}
```

### SyncState

```rust
pub enum SyncState {
    LocalOnly,
    Synced { at: DateTime<Utc> },
    LocalAhead,       // V2
    RemoteAhead,
    Conflict,         // V2
    SyncError { message: String, retries: u32 },
}
```

---

## Decisiones de Implementación Clave

### Storage (SQLite)

- WAL mode: OBLIGATORIO — sin él cada escritura hace fsync bloqueante
- FTS5 virtual table sobre `notes` para full-text search
- Tokenizador: `porter unicode61` — stemming básico
- Key de deduplicación: `UNIQUE(provider_id, source_id)`
- Layout en tabla separada (`note_layouts`) — se actualiza solo en mouse-up o close

### Security

- Tokens via `keyring` crate → Windows DPAPI
- NUNCA tokens en SQLite ni archivos de config
- OAuth2 para Keep: `keep.readonly` scope en V1
- Logs: nunca loguear contenido de notas ni tokens

### Sync Engine

- Pull-only en V1 (no write a providers)
- Backoff exponencial: start=5s, max=30min, multiplier=2.0, jitter=0.1
- Max retries por cycle: 5 — luego marca provider como Error hasta próximo startup/manual
- Tipos de trigger: Startup, Manual, Scheduled (default 15min), WakeFromSleep
- Un provider en error NO bloquea a los demás

### Windows Integration

- Autostart: `tauri-plugin-autostart` → HKCU registry (sin admin)
- Tray: siempre visible, click izquierdo = toggle ventana principal
- Close button: minimize to tray (NO exit del proceso)
- Exit real: solo via menú tray → "Salir"
- Sleep/Resume: cancelar sync en curso → esperar 10s al resume → re-trigger sync

---

## Google Keep API — Decisiones Específicas

- API: `notes.googleapis.com/v1` (oficial desde Mayo 2021)
- Auth: OAuth2, scope `https://www.googleapis.com/auth/keep.readonly`
- Redirect: Tauri OAuth plugin para browser redirect local
- Token: access_token + refresh_token en Windows Credential Manager
- Rate limit: respetar 429 + `Retry-After` header
- Limitaciones conocidas: sin imágenes/audio en contenido, sin PATCH, sin webhooks
- Si Keep falla: notas cached siguen visibles, SyncState::SyncError, retry con backoff
- La app NO colapsa si Keep no está disponible

---

## Criterios de Aceptación V1 (Métricas)

| Métrica | Target |
|---|---|
| RAM idle | < 50 MB |
| CPU idle | < 0.5% promedio |
| Startup time | < 1.0s |
| Search latency (1k notas) | < 50ms |
| Search latency (10k notas) | < 100ms |
| Arranque offline | Funcional < 1s, sin spinner |
| Recuperación error provider | < 5s para mostrar error en UI |
| Layout persistido tras reinicio | 100% de posiciones restauradas |
| Escrituras a disco en idle (5min) | 0 |

---

## Agent's Discretion

- Estructura interna de los Cargo.toml workspace — seguir convenciones de Tauri 2.x
- Naming de los IPC commands de Tauri — usar snake_case, prefijo por módulo (e.g., `notes_get_all`, `sync_trigger`)
- Estructura de los Solid.js stores — usar el patrón de stores del proyecto cuando se cree
- Testing approach para providers — usar mocks del trait `NoteProvider`, no providers reales en tests

---

## Deferred Ideas (no entran en V1)

- Editor inline de notas (V2)
- Write support a providers (V2)
- Provider Markdown/TXT (V2)
- Auto-update (V2)
- Firma de código / certificado OV (V2)
- Múltiples ventanas de nota abiertas simultáneamente (V2)
- Export a JSON/Markdown/PDF (V3)
- Plugin system para providers de terceros via WASM (V3)
- Notion / Obsidian providers (V3)
- Mobile companion (V3)
