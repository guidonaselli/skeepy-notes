# S10: Provider Status Dashboard

**Goal:** El usuario puede ver, de un vistazo, qué providers están activos, cuándo fue la
última sync, y si hay errores. Puede forzar una sync manual por provider.

**Demo:** Settings → sección "Providers" → lista con "Local (✓ Activo, sync hace 3 min)"
y "Google Keep (✓ Activo, sync hace 1h)" con botón "Sincronizar" por cada uno.

## Must-Haves

- Lista de todos los providers registrados con su estado actual
- Para cada provider: nombre, ícono/badge de stability, status (Activo/Error/Desconectado), timestamp de última sync
- Botón "Sincronizar ahora" por provider que dispara el sync de ese provider solamente
- El estado se actualiza en tiempo real via el evento `sync://progress` existente
- Error message visible cuando un provider está en estado Error

## Out of Scope

- Gráfico de historial de syncs
- Notificaciones del OS cuando un provider falla (M003+)

## Tasks

- [ ] **T01: Nuevo IPC `providers_status`**
  En `src-tauri/src/commands/sync.rs` (o nuevo `src/commands/providers.rs`):
  Retorna `Vec<ProviderStatusInfo>` con:
  ```rust
  pub struct ProviderStatusInfo {
      pub id: String,
      pub display_name: String,
      pub stability: ProviderStability,
      pub status: ProviderStatus,
      pub last_sync_at: Option<DateTime<Utc>>,
      pub last_error: Option<String>,
  }
  ```
  Lee el estado de `AppState.providers` + el `provider_sync_state` de la repo.

- [ ] **T02: Nuevo IPC `sync_provider`**
  En `sync.rs`: `sync_provider(state, app, provider_id: String)` — ejecuta sync
  solo para el provider especificado (no todos). Emite `sync://progress` igual que el sync global.

- [ ] **T03: Componente `ProviderStatusPanel`**
  En `src/components/ProviderStatusPanel.tsx`:
  - `createResource(providersStatus)` para cargar el estado inicial
  - Se suscribe a `sync://progress` para actualizar en tiempo real
  - Un `ProviderRow` por provider: nombre, stability badge, status icon, last_sync timestamp, botón Sync

- [ ] **T04: Integrar en Settings.tsx**
  Reemplazar la sección "Google Keep" estática por el `ProviderStatusPanel` dinámico.
  El panel muestra todos los providers que estén registrados.

- [ ] **T05: Actualizar `tauri.service.ts`**
  Agregar `providersStatus()` y `syncProvider(providerId: string)`.

## Files to Touch

- `src-tauri/src/commands/sync.rs` o nuevo `providers.rs`
- `src-tauri/src/lib.rs` — registrar nuevos commands
- `src/components/ProviderStatusPanel.tsx` — nuevo componente
- `src/components/Settings.tsx` — integrar panel
- `src/services/tauri.service.ts`
