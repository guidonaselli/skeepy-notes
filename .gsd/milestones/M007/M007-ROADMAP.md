# M007: V4.0 — "Multi-plataforma"

**Vision:** Skeepy sale de Windows y llega a macOS, Linux, iOS y Android.
La arquitectura basada en Tauri hace esto técnicamente posible sin reescribir el core.
Los desafíos son las integraciones específicas de cada plataforma: tray, autostart,
keychain, notificaciones y packaging.

**Success Criteria:**
- App funciona en macOS (M1/M2 + Intel) con behavior equivalente al de Windows
- App funciona en Ubuntu 22.04+ y Fedora 38+ con Wayland y X11
- Companion app iOS sincroniza notas locales con el desktop
- Companion app Android sincroniza notas locales con el desktop

---

## Slices

- [ ] **S32: macOS Port** `risk:medium` `depends:[M006 completo]`
  > After this: Skeepy funciona en macOS con tray en la menu bar, autostart via
  > LaunchAgent, tokens en macOS Keychain (via keyring crate — ya soportado),
  > y packaging como `.dmg` con notarización de Apple.
  > Ajustes UI para macOS: menu bar en vez de system tray de Windows.

- [ ] **S33: Linux Port** `risk:medium` `depends:[S32]`
  > After this: Skeepy funciona en Ubuntu, Fedora y Arch Linux con AppIndicator
  > para el tray, systemd service para autostart, y secret-service para credenciales
  > (keyring crate soporta libsecret en Linux).
  > Packaging: `.AppImage` y `.deb`.

- [ ] **S34: iOS Companion** `risk:very-high` `depends:[M006 completo]`
  > After this: Una app iOS (Tauri Mobile) puede ver las notas que están en la DB local
  > del desktop via un mecanismo de sync definido en este slice.
  > La sync desktop↔mobile requiere un protocolo de sincronización nuevo (probablemente
  > basado en un servidor local en la red LAN o iCloud Drive como intermediario).

- [ ] **S35: Android Companion** `risk:very-high` `depends:[S34]`
  > After this: Misma funcionalidad que iOS pero en Android.
  > Reutiliza el protocolo de sync de S34.

- [ ] **S36: Cloud Layout Sync** `risk:medium` `depends:[S34]`
  > After this: El layout de las notas (posición, tamaño) se sincroniza entre
  > dispositivos del mismo usuario. Requiere un backend mínimo (o usar iCloud/Google Drive
  > como key-value store). Las notas en sí NO se sincronizan via cloud — eso lo hacen
  > los providers. Solo el layout y las preferencias se sincronizan via cloud.

---

## Research Needed

### macOS Packaging y Notarización

- `cargo tauri build` ya genera `.dmg` para macOS cuando se corre en macOS
- Notarización Apple requiere cuenta Apple Developer ($99/año)
- Gatekeeper rechaza apps sin notarización — users tienen que hacer click derecho → Abrir
- Con notarización: instalación directa sin fricción
- Tauri 2.x soporta notarización en CI via `APPLE_CERTIFICATE` + `APPLE_SIGNING_IDENTITY` secrets

### Linux Tray

- `tauri-plugin-system-tray` usa `libayatana-appindicator` en Linux (Wayland/X11)
- Algunos entornos de escritorio (GNOME puro) no muestran el tray sin extensión — documentar esto
- `AppImage` es el formato más universal; `.deb` para distros basadas en Debian

### Mobile Sync Protocol

El mayor challenge de S34/S35 es el protocolo de sync desktop↔mobile.
Opciones:
1. **LAN sync directo:** el desktop expone un servidor HTTP local; la mobile app descubre
   el desktop via mDNS y sincroniza por WiFi. Sin cloud, sin cuenta.
   Pro: privacidad total. Con: requiere estar en la misma red.
2. **File-based sync via cloud storage:** exportar/importar un archivo JSON en
   iCloud Drive / Google Drive / OneDrive como intermediario.
   Pro: funciona offline (sync en diferido). Con: requiere que el usuario tenga
   uno de esos servicios configurado.
3. **Servidor propio:** Skeepy Sync Server (open source, self-hostable).
   Pro: más control. Con: requiere infraestructura.

**Recomendación:** Opción 1 (LAN sync) como V1 del companion, con Opción 2 como fallback.
Opción 3 para users avanzados en M008+.

---

## Decisiones Pendientes

- [ ] **D-M007-001:** ¿macOS build se hace en CI o requiere un Mac runner en GitHub Actions?
  GitHub Actions tiene `macos-latest` runners disponibles. Tauri 2.x los soporta.
  Agregar un job `build-macos` al `release.yml` es suficiente.

- [ ] **D-M007-002:** ¿iOS/Android con Tauri Mobile o con React Native / Flutter?
  Tauri Mobile está en alpha/beta para Tauri 2.x. Si no es suficientemente estable
  al momento de S34, evaluar Flutter como alternativa para el companion (compartiendo
  solo el protocolo de sync con el desktop).
