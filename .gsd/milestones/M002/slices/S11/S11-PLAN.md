# S11: GitHub Repo + README + Release pública

**Goal:** El repo es público, tiene un README que explica todo, y hay una release `v0.1.0`
con el instalador descargable.

**Demo:** Alguien encuentra el repo en GitHub, lee el README, descarga el `.exe`,
instala, conecta Keep y tiene sus notas en 5 minutos.

## Must-Haves

- README.md con: qué es, screenshot de la app, requisitos (Windows 10+), instalación paso a paso, setup de Keep, licencia MIT
- `CONTRIBUTING.md` mínimo: cómo buildear localmente
- GitHub Secrets configurados: `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`
- Tag `v0.1.0` pushed → CI genera release con el `.exe` adjunto
- El `.exe` instalado funciona en una máquina limpia (sin herramientas de dev)

## Out of Scope

- Documentación de la arquitectura interna (eso está en `.gsd/`)
- Traducciones del README

## Tasks

- [ ] **T01: README.md**
  Secciones:
  - Hero: qué es Skeepy (1 párrafo), screenshot
  - Requisitos: Windows 10+, WebView2 (preinstalado en Win10+)
  - Instalación: descargar `.exe` de Releases, ejecutar, aparece en tray
  - Conectar Google Keep: paso a paso (abrir Settings, sección Keep, Conectar)
  - Notas locales: dónde poner el `notes.json`
  - Datos y privacidad: qué se almacena y dónde (local solamente)
  - Licencia MIT

- [ ] **T02: CONTRIBUTING.md**
  - Prerequisites: Rust stable, Node 22, Tauri CLI v2
  - `npm install && cargo tauri dev` para levantar en modo dev
  - Cómo correr los tests: `cargo test --workspace`
  - Arquitectura en 1 párrafo con link a `.gsd/milestones/M001/M001-CONTEXT.md`

- [ ] **T03: GitHub Secrets**
  En el repo de GitHub → Settings → Secrets and variables → Actions:
  - `GOOGLE_CLIENT_ID`
  - `GOOGLE_CLIENT_SECRET`

- [ ] **T04: Push repo + tag**
  ```bash
  git init  # si no está inicializado
  git add .
  git commit -m "feat: V1.0 — Skeepy Notes initial release"
  git remote add origin https://github.com/<user>/skeepy-notes.git
  git push -u origin main
  git tag v0.1.0
  git push origin v0.1.0
  ```

- [ ] **T05: Verificar release**
  Confirmar que la GitHub Actions pipeline corre, genera el `.exe`, y lo adjunta al release.

## Files to Touch

- `README.md` (nuevo)
- `CONTRIBUTING.md` (nuevo)
