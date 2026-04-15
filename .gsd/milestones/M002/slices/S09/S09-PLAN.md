# S09: Keep OAuth Connect UI

**Goal:** El usuario puede conectar Google Keep desde Settings con un botón.
Sin pasos manuales, sin consola de dev tools.

**Demo:** Click "Conectar Google Keep" → browser se abre → usuario acepta permisos →
browser se cierra → Settings muestra "✓ Conectado".

## Must-Haves

- Botón "Conectar Google Keep" visible en Settings cuando Keep no está conectado
- Al hacer click: se levanta servidor local (tauri-plugin-oauth), se abre el browser con la URL de auth
- Al completar el auth en Google: los tokens se guardan y la UI actualiza a "✓ Conectado"
- Si el usuario cierra el browser sin completar: la UI vuelve al estado inicial sin crashear
- Verificación de `state` parameter para prevenir CSRF (comparar state generado vs recibido)
- Timeout si el usuario no completa el auth en 5 minutos
- Mensaje de error claro si `resolve_keep_credentials` falla (sin client_id configurado)

## Out of Scope

- Refresh automático de tokens desde la UI (eso ya lo hace KeepProvider internamente)
- Revocar desde múltiples dispositivos (fuera del scope OAuth de V1)

## Tasks

- [x] **T01:** tauri-plugin-oauth + tauri-plugin-shell — npm + Cargo.toml + lib.rs + capabilities/default.json ✓
- [x] **T02:** keep_start_auth ya correcto — toma redirect_uri, resuelve credentials internamente ✓
- [x] **T03:** keep_complete_auth ya correcto — toma code, code_verifier, redirect_uri ✓
- [x] **T04:** handleKeepConnect() en Settings.tsx — orquesta: oauthStart → keepStartAuth → shellOpen → listen oauth://url → CSRF check → keepCompleteAuth → oauthCancel ✓
- [x] **T05:** Estados idle/connecting/error con timeout 5min en Settings.tsx ✓
- [x] **T06:** tauri.service.ts ya tenía las firmas correctas ✓
- [x] **T07:** 4 tests de resolve_keep_credentials. 53 tests workspace OK ✓

## Files to Touch

- `src-tauri/Cargo.toml` — add tauri-plugin-oauth
- `src-tauri/tauri.conf.json` — add plugin to allowlist
- `src-tauri/src/lib.rs` — register plugin + new commands
- `src-tauri/src/commands/keep.rs` — T02, T03
- `src/components/Settings.tsx` — T04, T05
- `src/services/tauri.service.ts` — T06

## Research Notes

`tauri-plugin-oauth` (https://github.com/FabianLars/tauri-plugin-oauth):
- `oauth::start(config, handler)` → levanta un servidor HTTP en un puerto libre
- El handler recibe la URL completa del callback (con `?code=...&state=...`)
- `oauth::cancel(port)` para apagar el servidor después de recibir el callback
- El port se obtiene del `OauthConfig` o se asigna automáticamente

Para abrir el browser: `tauri-plugin-shell` → `open(url)`.
Ya está en el workspace de plugins de Tauri 2.x, no requiere instalación extra si ya usamos otros plugins del workspace.
