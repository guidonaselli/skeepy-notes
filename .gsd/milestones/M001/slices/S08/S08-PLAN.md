# S08: NSIS Installer + Release Pipeline

**Goal:** `cargo tauri build` produce un `.exe` instalador NSIS funcional, instalación limpia sin admin, uninstall limpio. GitHub Actions genera el release al hacer push de un tag `v*`.
**Demo:** Tag `v0.1.0` → GitHub Release con un `.exe` descargable → instalación en carpeta user → app en system tray → uninstall limpio.

## Must-Haves

- `cargo tauri build` compila sin errores en CI (windows-latest)
- Installer `.exe` instala en `%LOCALAPPDATA%\Programs\Skeepy` (sin admin)
- Shortcut en Start Menu
- Uninstall desde Configuración → Aplicaciones limpia todo
- GitHub Actions workflow: `.github/workflows/release.yml`
  - Trigger: push de tag `v[0-9]+.[0-9]+.[0-9]+`
  - Steps: checkout → setup node+rust → npm ci → tests → tauri build → GitHub Release
- CI job para PRs: typescript check + vite build + cargo check + tests
- `tauri.conf.json` NSIS config: `installMode: "currentUser"` (no admin)

## Out of Scope

- Code signing (V2 — needs paid cert)
- Auto-update (V2 — `tauri-plugin-updater`)
- macOS / Linux builds

## Tasks

- [x] **T01: GitHub Actions release.yml** — ✓ Done
- [x] **T-credentials-1: Hardcodear client_id en binario** — `keep_start_auth` y `keep_complete_auth` ya no reciben client_id por IPC. `resolve_keep_credentials` lee settings DB → `option_env!("GOOGLE_CLIENT_ID")`. ✓ Done
- [x] **T-credentials-2: BYO credentials en Settings UI** — Campos "Client ID" y "Client Secret" en sección avanzada de Google Keep. `keep_credentials_get` / `keep_credentials_set` IPC. ✓ Done
- [x] **T02: tauri.conf.json bundle** — NSIS `installMode: "currentUser"` ✓. `targets: "all"` produce NSIS en Windows. ✓ Done
- [x] **T03: Icons** — `icon.ico`, `32x32.png`, `128x128.png` presentes. ✓ Done
- [x] **T04: Verify `cargo tauri build` config** — productName "Skeepy", identifier "com.skeepy.notes", bundle targets "all". `GOOGLE_CLIENT_ID`/`GOOGLE_CLIENT_SECRET` ahora se pasan al step de build en release.yml. ✓ Done
- [ ] **T05: `cargo tauri build` manual** — Requiere `cargo install tauri-cli --version "^2"` y WebView2 SDK en la máquina del dev. Paso manual.

## Files Touched

- `.github/workflows/release.yml` ✓
- `src-tauri/tauri.conf.json` — ya tiene NSIS config
