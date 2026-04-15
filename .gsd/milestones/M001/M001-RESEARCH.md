# M001: V1 — Research

**Researched:** 2026-04-12
**Domain:** Desktop App (Rust/Tauri), Google Keep API, SQLite FTS5
**Confidence:** HIGH (APIs verificadas, stack evaluado contra alternativas reales)

---

## Summary

Skeepy es una app de escritorio Windows corriendo 24/7. El requisito central — consumo casi nulo en idle — elimina inmediatamente Electron (GC de V8 + Chromium completo = 150-250MB RAM). La elección de Rust + Tauri 2.x da el menor footprint posible con una UI web mantenible por contributors sin Rust.

Google Keep tiene una API oficial (`notes.googleapis.com/v1`) viable para read-only en V1. Sus limitaciones (sin imágenes, sin PATCH, sin webhooks) son manejables en el diseño. El riesgo real es que Google la deprece — mitigado haciendo Keep un provider opcional con `ProviderStability::Experimental`.

SQLite con FTS5 maneja search en 10k notas en < 10ms sin dependencias extra. WAL mode es obligatorio para eliminar fsync por escritura. Windows Credential Manager (DPAPI) via `keyring` crate es el único almacenamiento aceptable para tokens OAuth.

---

## Don't Hand-Roll

| Problema | No construir | Usar en cambio | Por qué |
|---|---|---|---|
| Full-text search | Motor de search propio | SQLite FTS5 nativo | FTS5 viene con SQLite, zero-config, suficiente para miles de notas |
| Token storage | Cifrado propio | `keyring` crate (DPAPI) | DPAPI es el estándar del OS, auditado, sin re-inventar criptografía |
| Async trait objects | Workarounds manuales | `async-trait` crate | Rust no soporta async en traits nativamente (aún) |
| Tauri tray | Librerías de terceros | `tauri-plugin-system-tray` | Plugin oficial, mantenido, evita incompatibilidades |
| Autostart Windows | Escribir al registry directamente | `tauri-plugin-autostart` | Plugin oficial con fallback por plataforma |
| OAuth2 flow | Implementar PKCE manualmente | `tauri-plugin-oauth` + `oauth2` crate | PKCE + local redirect handler ya implementado |
| DB migrations | SQL hardcodeado en runtime | `sqlx migrate` | Migrations versionadas, rollback, compile-time checks |
| Rate limiting | Sleep loops manuales | `governor` crate | Rate limiter token-bucket correcto, sin busy-wait |

---

## Common Pitfalls

### Pitfall 1: SQLite sin WAL mode
**What goes wrong:** Cada escritura hace un `fsync()` bloqueante. En una app que actualiza layout en mouse-move, esto genera I/O visible y degrada performance.
**Why it happens:** SQLite default es journal mode = DELETE (un WAL file por DB).
**How to avoid:** `PRAGMA journal_mode=WAL;` como primera operación post-conexión.
**Warning signs:** `disk_io` metrics altos en idle, lag en mouse interactions.

### Pitfall 2: Polling activo del sync engine
**What goes wrong:** Un timer de 30s que siempre ejecuta fetch HTTP aunque no haya nada nuevo. CPU y red innecesarios 24/7.
**Why it happens:** Es la implementación más simple. Se escala mal en 8h de uso.
**How to avoid:** Check de cooldown (min 60s entre syncs del mismo provider), skip si provider está en backoff, no ejecutar si app está en estado "suspended".
**Warning signs:** CPU > 1% sostenido en idle, conexiones HTTP en idle en el network monitor.

### Pitfall 3: Tokens en SQLite settings table
**What goes wrong:** Tokens en texto plano (o base64) en la DB son legibles por cualquier proceso con acceso al archivo.
**Why it happens:** Es la ruta fácil cuando ya tenés SQLite abierto.
**How to avoid:** `keyring` crate solamente. DPAPI cifra con la clave del usuario del OS.
**Warning signs:** Cualquier `INSERT INTO settings VALUES ('keep_token', ...)`.

### Pitfall 4: UI acoplada directamente al provider
**What goes wrong:** La UI llama a `KeepProvider.fetch()` directamente. Cuando Keep falla, la UI crashea. Cuando Keep desaparece, hay que reescribir la UI.
**Why it happens:** Shortcut de arquitectura para "llegar rápido".
**How to avoid:** UI solo lee del `NoteService` (que lee de SQLite). `NoteProvider` es transparente para la UI.
**Warning signs:** Imports de `keep_provider` en archivos de UI.

### Pitfall 5: Ignorar el close behavior
**What goes wrong:** La app se cierra con X y el usuario pierde la funcionalidad residente. O peor: el proceso muere con datos sin flush.
**Why it happens:** Tauri por default cierra la ventana Y el proceso con X.
**How to avoid:** `api.prevent_close()` en el event handler de `CloseRequested`, redirigir a `window.hide()`.
**Warning signs:** App no aparece en el tray después de cerrar la ventana.

### Pitfall 6: SyncOrchestrator bloqueante en startup
**What goes wrong:** La app no muestra nada hasta que termina el primer sync. Si Keep está lento, el usuario espera 10s para ver su tray icon.
**Why it happens:** `await sync_all_providers()` antes de `show_window()`.
**How to avoid:** Startup sync es async y fire-and-forget. La UI muestra las notas cacheadas inmediatamente y actualiza cuando llega el sync.
**Warning signs:** Startup time > 1s, spinner en la ventana principal durante boot.

### Pitfall 7: Re-renders innecesarios en Solid.js
**What goes wrong:** Actualizar el store de notas completo en cada tick de sync causa que todos los NoteCards re-rendericen aunque no hayan cambiado.
**Why it happens:** Pasar arrays completos como signal en lugar de signals granulares por nota.
**How to avoid:** Map de `NoteId → Signal<Note>` en el store. Solo la nota que cambió actualiza su signal.
**Warning signs:** Jank visible al recibir sync results con muchas notas.

---

## Relevant Code

Proyecto nuevo — sin código existente. Refs externas relevantes:

- Tauri 2.x docs: https://v2.tauri.app/
- `notes.googleapis.com` REST API: https://developers.google.com/keep/api/reference/rest
- SQLite FTS5: https://www.sqlite.org/fts5.html
- `keyring` crate: https://docs.rs/keyring/latest/keyring/
- `sqlx` migrations: https://docs.rs/sqlx/latest/sqlx/macro.migrate.html
- `tauri-plugin-autostart`: https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/autostart
- `tauri-plugin-oauth`: https://github.com/FabianLars/tauri-plugin-oauth

## Google Keep API — Findings Específicos

**Endpoint base:** `https://keep.googleapis.com/v1`

**Operaciones disponibles V1:**
- `GET /notes` — lista notas (paginado con `pageToken`, pageSize max 1000)
- `GET /notes/{name}` — nota individual (name = "notes/{id}")
- `POST /notes` — crear nota (texto o checklist)
- `DELETE /notes/{name}` — eliminar nota
- `GET /notes/{name}/permissions` — permisos de la nota
- `GET /labels` — listar labels
- `POST /labels` — crear label
- `PATCH /labels/{name}` — actualizar label
- `DELETE /labels/{name}` — eliminar label

**Campo `updateTime` en `notes.list`:** Disponible — permite sync incremental si el provider filtra por este campo. La API NO tiene un parámetro `modifiedAfter` nativo, hay que filtrar client-side.

**Limitación importante:** La API no soporta watch/webhooks. El único approach es polling periódico + comparación de `updateTime`.

**Nota sobre el campo `name`:** El ID nativo de una nota es `notes/{random_id}`. Usar como `source_id` en la entidad local.

**OAuth2 scopes necesarios:**
- V1: `https://www.googleapis.com/auth/keep.readonly`
- V2: `https://www.googleapis.com/auth/keep`

**Testing sin verificación OAuth:** Hasta 100 "test users" en la Google Cloud Console sin pasar por la verificación de la app. Suficiente para desarrollo y early adopters.

---

## OAuth2 Distribution Architecture

### El modelo correcto para una desktop app open source

OAuth2 no tiene costo por usuario. Google no cobra por cuánta gente autentica con tu app. El costo de infraestructura es CERO porque la arquitectura es completamente serverless:

```
Flow PKCE (ya implementado en Skeepy):

1. App genera code_verifier + code_challenge localmente
2. App abre browser → https://accounts.google.com/o/oauth2/v2/auth?...
3. Usuario logea en los servidores de Google (Skeepy nunca ve la contraseña)
4. Google redirige a http://localhost:PORT?code=...
5. App intercambia code por tokens (POST a google, no hay server propio)
6. Tokens → Windows Credential Manager (DPAPI)
```

No hay backend. No hay servidor. No hay base de datos central. Escala a millones de usuarios sin cambiar arquitectura.

### client_id en el binario — por qué es correcto

Google documenta esto explícitamente bajo "Installed Application" OAuth2:

> "It is not possible to keep secrets in installed applications. The client_secret is considered public for installed applications."

PKCE compensa esto. El `client_secret` en el binario de una desktop app es el diseño CORRECTO según la especificación OAuth2 para installed apps (RFC 8252).

Lo que hace el developer:
- Registra UNA app en Google Cloud Console (gratis)
- Obtiene UN client_id (y opcionalmente client_secret)
- Los compila en el binario via env vars en CI

Lo que tiene cada usuario:
- SUS tokens (access + refresh) en SU Windows Credential Manager
- El developer nunca ve, toca ni almacena datos de usuarios

### Requerimientos antes de release pública

**Google OAuth Verification** — obligatorio para scopes sensibles en apps públicas:

| Etapa | Usuarios permitidos | Pantalla para el usuario |
|-------|--------------------|-----------------------------|
| Sin verificar | Hasta 100 "test users" agregados manualmente en Google Cloud Console | ⚠️ "Google no verificó esta app" — el usuario debe hacer click en Avanzado → Continuar |
| Verificado | Ilimitados | Pantalla de permisos normal y limpia |

El proceso de verificación es gratis pero burocrático:
1. Política de privacidad pública (URL requerida)
2. Dominio propio verificado
3. Descripción clara del uso de cada scope
4. Review manual de Google (2-6 semanas típicamente)
5. Posible video demo o cuestionario

**Para desarrollo y early access:** los 100 test users son suficientes. Iniciar el proceso de verificación en paralelo con el desarrollo de S07/S08.

### BYO Credentials (override para power users)

Patrón: el app usa el client_id compilado por default, pero permite override en Settings.

```rust
// Resolución de credenciales (en orden de prioridad):
// 1. Override del usuario en Settings DB
// 2. client_id/secret compilado en el binario via env!()
pub async fn resolve_keep_credentials(repo: &dyn SettingsRepository) -> (String, Option<String>) {
    let custom_id = repo.get_raw("keep_client_id").await.ok().flatten();
    match custom_id {
        Some(id) if !id.is_empty() => {
            let secret = repo.get_raw("keep_client_secret").await.ok().flatten();
            (id, secret)
        }
        _ => (KEEP_CLIENT_ID.to_string(), Some(KEEP_CLIENT_SECRET.to_string()))
    }
}
```

**Estado actual del código:** `keep_start_auth` recibe `client_id` como parámetro IPC desde el frontend — esto es incorrecto para producción. **Antes de la primera release pública**, refactorizar para que el backend resuelva las credenciales internamente via `resolve_keep_credentials`.

### Pitfall 8: OAuth verification no hecha antes de release pública

**What goes wrong:** El 100% de los usuarios ve "Esta app no es de confianza" de Google. Muchos abandonan antes de completar la autenticación.
**Why it happens:** El proceso de verificación se ignora o se deja para después.
**How to avoid:** Iniciar el proceso de verificación de Google OAuth tan pronto como el scope `keep.readonly` sea necesario. Requiere privacy policy URL, dominio verificado y review manual. Planear 4-6 semanas.
**Warning signs:** La pantalla de consent de Google muestra "unverified app" en lugar del nombre de la app.

---

## Microsoft Store Distribution

### Qué cambia para el Store

La diferencia técnica vs la release NSIS directa:

| Aspecto | NSIS (release directa) | MSIX (Microsoft Store) |
|---------|----------------------|----------------------|
| Packaging | `.exe` NSIS | `.msix` via WiX bundler |
| Instalación | %LOCALAPPDATA%\Programs | Sandbox del Store |
| Admin requerido | No (currentUser) | No (siempre user) |
| OAuth callback | localhost:PORT ✓ | localhost:PORT ✓ |
| Auto-update | Manual (S02 backlog) | Store lo maneja |
| Certificación | Ninguna | 1-3 días hábiles |
| Costo one-time | Gratis | ~$20 cuenta Partner Center |

### Cambio en tauri.conf.json para MSIX

```json
"bundle": {
  "targets": ["nsis", "msi"],
  "windows": {
    "nsis": { "installMode": "currentUser" },
    "wix": {
      "language": "es-ES"
    }
  }
}
```

El flow de OAuth con `localhost` callback funciona dentro del sandbox del Store sin cambios adicionales.

### Requerimientos de certificación Microsoft

- App no puede tener admin prompt (✓ ya cumplido con `installMode: currentUser`)
- No puede modificar archivos fuera de su directorio y AppData (✓ Skeepy solo escribe en AppData)
- Debe declarar capabilities en manifest (Tauri lo genera automáticamente)
- Proceso de review: 1-3 días hábiles típicamente (mucho más rápido que Google)

---

## Sources

- Google Keep API Reference: developers.google.com/keep (HIGH confidence — documentación oficial)
- Tauri 2.x stable: github.com/tauri-apps/tauri — lanzado Octubre 2024 (HIGH confidence)
- SQLite FTS5: sqlite.org/fts5.html (HIGH confidence — documentación oficial)
- keyring crate: Windows DPAPI verificado en docs.rs/keyring (HIGH confidence)
- Análisis de alternativas: basado en benchmarks públicos de Tauri vs Electron (MEDIUM confidence — varían por hardware, son ballpark correctos)
