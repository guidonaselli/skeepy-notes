# S06: Google Keep Provider

**Goal:** Implementar `skeepy-provider-keep` — KeepProvider que autentica via OAuth2 PKCE, llama a `notes.googleapis.com/v1`, almacena tokens en DPAPI/keyring, y satisface el trait `NoteProvider` con `stability: Experimental`.
**Demo:** El usuario puede hacer click en "Conectar Google Keep", autenticarse en el browser, y sus notas de Keep aparecen mezcladas con las locales en la UI.

## Must-Haves

- `KeepProvider` implementa `NoteProvider` con `stability: Experimental`
- OAuth2 PKCE flow via `tauri-plugin-oauth` (redirect a localhost callback)
- Token (access_token + refresh_token) almacenado via `keyring` crate (DPAPI en Windows)
- `reqwest` client para `notes.googleapis.com/v1/notes` y `notes.googleapis.com/v1/notes/{name}`
- Rate limiter via `governor` crate — respeta límites de la API de Google
- `fetch_notes(since)` — usa `filter` param si `since` es Some (incremental sync)
- Mapping `Note` → `RemoteNote`: title, text/checklist, labels, colors, pinned, archived, timestamps
- Error handling: `ProviderError::AuthRequired` cuando el token expira, `ProviderError::Api` para errores HTTP
- `capabilities()` reporta `can_read: true, supports_incremental_sync: true, stability: Experimental`
- Tests: mock HTTP responses para list/get, token storage mock, color mapping

## Google Keep API specifics

- Base URL: `https://notes.googleapis.com/v1`
- Auth scope: `https://www.googleapis.com/auth/keep.readonly`
- List: `GET /v1/notes?pageSize=100&filter=updateTime>"{rfc3339}"`
- Get: `GET /v1/notes/{name}`
- Note structure: `{ name, title, body: { text | list }, labels, color, pinned, archived, trashTime, createTime, updateTime }`
- Colors: `DEFAULT, RED, ORANGE, YELLOW, GREEN, TEAL, BLUE, GRAY, PURPLE, BROWN, PINK, CYAN`
- Pagination: `nextPageToken` field

## Out of Scope

- Write operations (V2)
- `CYAN` color (no equivalent in Skeepy — maps to `Teal`)
- Note images/audio attachments

## Tasks

- [ ] **T01: Cargo.toml** — agregar reqwest, oauth2, keyring, governor
- [ ] **T02: Keep API client** (api.rs) — structs de respuesta, HTTP client, pagination
- [ ] **T03: Token storage** (token.rs) — CRUD via keyring crate
- [ ] **T04: OAuth2 flow** (auth.rs) — PKCE, redirect local, refresh token
- [ ] **T05: KeepProvider** (provider.rs) — impl NoteProvider, fetch_notes, fetch_note
- [ ] **T06: Color + content mapping** — KeepNote → RemoteNote
- [ ] **T07: IPC commands** — `keep_connect`, `keep_disconnect`, `keep_status` en skeepy-app
- [ ] **T08: Tests** — mock HTTP, color mapping, pagination

## Files Likely Touched

- `src-tauri/crates/providers/keep/Cargo.toml`
- `src-tauri/crates/providers/keep/src/lib.rs`
- `src-tauri/crates/providers/keep/src/api.rs`
- `src-tauri/crates/providers/keep/src/token.rs`
- `src-tauri/crates/providers/keep/src/auth.rs`
- `src-tauri/crates/providers/keep/src/provider.rs`
- `src-tauri/Cargo.toml` — agregar skeepy-provider-keep
- `src-tauri/src/commands/keep.rs` — IPC commands
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`
