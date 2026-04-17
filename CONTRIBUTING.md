# Contribuir a Skeepy

## Requisitos

- [Rust stable](https://rustup.rs/) (1.75+)
- [Node.js](https://nodejs.org/) 22+
- [Tauri CLI v2](https://v2.tauri.app/start/prerequisites/): `cargo install tauri-cli --version "^2"`
- **Windows:** WebView2 (preinstalado en Windows 10+)
- **macOS:** Xcode Command Line Tools
- **Linux:** `libwebkit2gtk-4.1`, `libgtk-3`, `libayatana-appindicator3`

## Levantar el entorno de desarrollo

```bash
# Instalar dependencias del frontend
npm install

# Levantar en modo dev (hot-reload frontend + backend compilado en debug)
cargo tauri dev
```

La ventana de Skeepy se abre automáticamente. Los cambios en `src/` se reflejan al instante. Los cambios en `src-tauri/` requieren reinicio del proceso Rust (automático).

## Correr los tests

```bash
# Tests de todos los crates del workspace
cargo test --workspace

# TypeScript check
npx tsc --noEmit
```

## Arquitectura

```
src-tauri/crates/core/                → Domain layer (traits, entidades, sin I/O)
src-tauri/crates/storage/             → SQLite + FTS5 + migraciones (implementa repos del domain)
src-tauri/crates/providers/
  skeepy-provider-local/              → Notas locales (JSON en AppData)
  skeepy-provider-keep/               → Google Keep (OAuth2 PKCE, keep.readonly)
  skeepy-provider-onenote/            → Microsoft OneNote (PKCE, Graph API)
  skeepy-provider-notion/             → Notion (OAuth2, Basic auth)
  skeepy-provider-markdown/           → Carpeta Markdown local
  skeepy-provider-obsidian/           → Obsidian Vault (recursive walk, backlinks, tags)
  skeepy-provider-sticky-notes/       → Windows Sticky Notes (plum.sqlite, solo Windows)
src-tauri/src/                        → Tauri shell (IPC commands, state, tray, autostart)
src-tauri/src/semantic/               → TF-IDF vectorizer + indexación en background
src-tauri/src/smart_sync.rs           → Scheduler adaptativo basado en historial de uso
src/                                  → Solid.js frontend
```

## Agregar un nuevo provider

1. Crear un nuevo crate en `src-tauri/crates/providers/<nombre>/`
2. Implementar el trait `NoteProvider` de `skeepy-core`
   - `fetch_notes()` es obligatorio
   - `update_note()`, `delete_note()`, `create_note()` son opcionales (default = `NotSupported`)
3. Registrar el provider en `src-tauri/src/lib.rs` (en el `setup`)
4. Agregar IPC commands de autenticación si el provider los necesita
5. Agregar la sección correspondiente en `src/components/Settings.tsx`

## Variables de entorno

Para compilar con credenciales embebidas (opcional):

```bash
# Google Keep
GOOGLE_CLIENT_ID=tu-client-id GOOGLE_CLIENT_SECRET=tu-secret cargo tauri build

# Azure (OneNote)
AZURE_CLIENT_ID=tu-client-id cargo tauri build

# Notion
NOTION_CLIENT_ID=tu-client-id NOTION_CLIENT_SECRET=tu-secret cargo tauri build
```

Sin estas variables, el binario compila igual — los usuarios ingresan sus propias credenciales en Settings.

## Build de producción

```bash
# Genera el installer en src-tauri/target/release/bundle/
cargo tauri build
```

El CI (`.github/workflows/`) genera builds para Windows x64, macOS x64, macOS ARM y Linux en cada tag.

## Convenciones

- Commits en [Conventional Commits](https://www.conventionalcommits.org/)
- Un PR = un concern
- Cada PR debe pasar `cargo test --workspace` y `npx tsc --noEmit`
- No agregar dependencias sin justificación — revisar primero si el problema se puede resolver con lo que ya hay en el workspace
