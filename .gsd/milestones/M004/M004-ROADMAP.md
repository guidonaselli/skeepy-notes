# M004: V2.5 — "Escribí desde Skeepy"

**Vision:** Skeepy deja de ser solo un lector y se convierte en un editor. El usuario puede
crear notas locales, editarlas, y tener write bidireccional con Google Keep. La app se mantiene
sola (auto-update), y el instalador ya no dispara alertas de SmartScreen.

**Success Criteria:**
- El usuario puede crear una nota nueva local desde la UI con Ctrl+N
- Editar una nota existente con un editor inline simple (texto + checklist)
- Crear y editar notas en Google Keep directamente desde Skeepy
- La app se actualiza sola cuando hay una nueva versión disponible
- El instalador está firmado — SmartScreen no muestra "Publisher Unknown"

---

## Slices

- [ ] **S18: Write Support — Local Provider** `risk:medium` `depends:[S15]`
  > After this: El usuario puede crear una nota nueva local (Ctrl+N o botón +),
  > editar el título y cuerpo, y eliminar notas locales. Los cambios se persisten
  > en SQLite y se reflejan en el notes.json (bidireccional local).

- [ ] **S19: Inline Editor** `risk:medium` `depends:[S18]`
  > After this: Existe un editor inline en el Note Detail View para notas locales.
  > Soporta texto plano, formato básico (negrita, cursiva via Markdown shortcuts),
  > y edición de checklists (toggle, add item, delete item).
  > Guardar con Ctrl+S o al perder foco. Descartar con Escape.

- [ ] **S20: Write Support — Google Keep** `risk:high` `depends:[S19]`
  > After this: El usuario puede crear y actualizar notas en Google Keep desde Skeepy.
  > Usa la API `POST /notes` y `PATCH /notes/{name}` (cuando esté disponible).
  > Maneja conflictos: si la nota fue modificada remotamente, muestra el diff y pide
  > resolución al usuario.
  > Nota: requiere cambiar el scope OAuth a `keep` (no solo `keep.readonly`) y
  > re-autenticación de usuarios existentes.

- [ ] **S21: Auto-Update** `risk:low` `depends:[S11]`
  > After this: Al abrir la app, verifica en background si hay una nueva versión en GitHub Releases.
  > Si la hay, muestra un badge en el tray y un aviso en Settings con botón "Actualizar ahora".
  > La actualización se descarga en background y se instala al próximo inicio.
  > Usa `tauri-plugin-updater`.

- [ ] **S22: Code Signing** `risk:low` `depends:[S21]`
  > After this: El instalador `.exe` está firmado con un certificado OV (Organization Validation).
  > SmartScreen no muestra "Publisher Unknown". El certificado se guarda como GitHub Secret
  > y se usa en el CI pipeline de release.
  > Costo: ~$200-500/año (DigiCert, Sectigo, etc.).

---

## Research Needed

### Write Support — Local Provider

El `LocalProvider` actual es read-only: lee desde `notes.json` y no escribe de vuelta.
Para write support local, el flujo sería:
1. El usuario crea/edita una nota → se guarda en SQLite (inmediato)
2. Al guardar, se escribe de vuelta al `notes.json` (o se abandona el JSON como source of truth
   y SQLite se convierte en el storage permanente para notas locales)

**Decisión pendiente:** ¿SQLite como storage definitivo para notas locales, o mantener
notes.json como source of truth y sincronizar bidireccional? Recomendación: SQLite es el
storage definitivo desde V2.5 para notas locales. El notes.json pasa a ser solo un formato
de importación/exportación.

### Write Support — Google Keep

La API `keep.googleapis.com/v1` soporta:
- `POST /notes` — crear nota (texto o checklist)
- `DELETE /notes/{name}` — eliminar nota
- **No soporta `PATCH /notes/{name}`** — las notas existentes NO se pueden editar via API oficial

Esto es una limitación crítica: Keep API V1 no permite editar notas. Opciones:
1. Solo permitir crear notas nuevas y eliminar notas en Keep (no editar)
2. Implementar "edit = delete + create" (destructivo, pierde metadata)
3. Esperar a que Google agregue PATCH (sin garantías de timeline)

**Decisión para S20:** Solo crear y eliminar notas en Keep. La edición de notas Keep
en Skeepy quedará bloqueada con un mensaje claro ("Esta nota es de Google Keep y no puede
editarse desde Skeepy — editala en la app de Keep").

### Auto-Update

`tauri-plugin-updater` soporta GitHub Releases como endpoint.
El updater lee un archivo `latest.json` (generado por tauri-action) con la versión y URL del asset.
Configurar en `tauri.conf.json`:
```json
"updater": {
  "active": true,
  "endpoints": ["https://github.com/<user>/skeepy-notes/releases/latest/download/latest.json"],
  "dialog": false,
  "pubkey": "..."
}
```
El `pubkey` es la clave pública de un par de claves que se genera una vez y se guarda como secret en GitHub.

### Code Signing Windows

El proceso para firmar el instalador NSIS:
1. Comprar certificado OV (no EV — EV requiere hardware token y es más caro)
2. El certificado llega como `.pfx` con contraseña
3. En CI: `TAURI_PRIVATE_KEY` + `TAURI_KEY_PASSWORD` como GitHub Secrets
4. tauri-action firma el `.exe` automáticamente si los secrets están configurados

Con OV cert: SmartScreen deja de decir "Publisher Unknown" pero puede seguir mostrando
advertencia hasta que la app acumule reputación (típicamente 1-3 meses de descargas).
Con EV cert: SmartScreen la aprueba inmediatamente pero requiere hardware token físico.

---

## Breaking Change: OAuth Scope

S20 requiere cambiar el scope de `keep.readonly` a `keep`.
Esto rompe la autenticación de todos los usuarios existentes — necesitarán re-autenticar.
Estrategia: mostrar un aviso "Para habilitar la edición de notas de Keep, necesitamos
un permiso adicional. Hacé click aquí para re-conectar tu cuenta de Google."
El re-auth debe ser un flujo suave, no un rompimiento abrupto.
