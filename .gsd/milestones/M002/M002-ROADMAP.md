# M002: V1.1 — "La app que cualquiera puede usar"

**Vision:** Cerrar el gap entre el código funcionando y una app que un usuario no-técnico
pueda instalar, conectar a Google Keep y usar sin leer documentación. El flujo OAuth completo
funciona desde la UI, el estado de los providers es visible, y existe una release pública
en GitHub con README claro.

**Success Criteria:**
- Un usuario puede conectar Google Keep en < 3 pasos desde Settings
- El estado de cada provider (activo, error, cuándo fue la última sync) es visible en la UI
- El repo es público en GitHub con README que explica instalación y setup de Keep
- Primera release `v0.1.0` publicada con el instalador NSIS descargable
- Proceso de Google OAuth verification iniciado formalmente

---

## Slices

- [ ] **S09: Keep OAuth Connect UI** `risk:medium` `depends:[S08]`
  > After this: El usuario puede hacer click en "Conectar Google Keep" en Settings,
  > el browser se abre con la URL de auth, Google redirige al callback local,
  > y los tokens se guardan automáticamente. Sin pasos manuales.

- [ ] **S10: Provider Status Dashboard** `risk:low` `depends:[S09]`
  > After this: Una sección en Settings (o en la barra de la UI) muestra el estado
  > de cada provider registrado: activo/error/desconectado, última sync timestamp,
  > y un botón para forzar resync manual.

- [ ] **S11: GitHub Repo + README + Release pública** `risk:low` `depends:[S08]`
  > After this: El repo está en GitHub, el README explica cómo instalar y conectar Keep,
  > el tag `v0.1.0` dispara la pipeline y genera el instalador descargable.

- [ ] **S12: Google OAuth Verification** `risk:low` `depends:[S11]`
  > After this: El proceso formal de verificación de Google OAuth está iniciado
  > (privacy policy URL, dominio verificado, descripción de scopes enviada a Google).
  > Este slice es mayormente proceso burocrático, no código.

---

## Boundary Map

### S09 — Keep OAuth Connect UI

Consumes (ya existe):
- `keep_start_auth` IPC — toma `redirect_uri`, devuelve `auth_url` + `code_verifier` + `state`
- `keep_complete_auth` IPC — toma `code`, `code_verifier`, `redirect_uri`
- `tauri-plugin-oauth` — levanta servidor local para capturar el callback de Google
- `tauri-plugin-shell` — abre el browser con la auth URL

Produce:
- Botón "Conectar Google Keep" en Settings.tsx
- Handler que orquesta: start server → build URL → open browser → wait callback → complete auth
- Estado de UI: loading / error / success
- Guarda `code_verifier` + `state` en memoria del componente para verificar CSRF

### S10 — Provider Status Dashboard

Consumes:
- `keep_status` IPC (ya existe)
- Nuevo IPC `providers_status` — retorna `Vec<ProviderStatusInfo>` con id, nombre, status, last_sync_at

Produce:
- Componente `ProviderStatusPanel` en Settings
- Estado visual: ✓ Activo / ⚠ Error / ✗ Desconectado
- Timestamp de última sync
- Botón "Sincronizar ahora" por provider

### S11 — GitHub Repo + README

Produce:
- `README.md` con: qué es Skeepy, screenshots, cómo instalar, cómo conectar Keep, licencia
- `.github/` workflows ya existen — solo verificar que CI pasa con repo público
- Tag `v0.1.0` pushed → primera release

### S12 — Google OAuth Verification

Produce (no código):
- Privacy policy URL (puede ser un GitHub Pages simple)
- Dominio verificado en Google Search Console
- Formulario de verificación OAuth enviado a Google
- Entrada en DECISIONS.md documentando el estado del proceso
