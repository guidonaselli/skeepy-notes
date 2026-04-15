# Skeepy — Master Roadmap

**Última actualización:** 2026-04-13
**Milestone activo:** M001 (completando S08) → M002

---

## Visión del producto

Skeepy arranca como un visor de notas local con soporte Google Keep y evoluciona
hasta convertirse en la plataforma de notas más completa para Windows — y eventualmente
multiplataforma — con write support bidireccional, plugin system abierto, e IA local.

**Principio central que nunca cambia:** el storage local es siempre la source of truth.
Los providers son fuentes que se sincronizan con la DB local. La app funciona offline
sin excepción.

---

## Milestones

| # | Versión | Nombre | Estado |
|---|---------|--------|--------|
| M001 | V1.0 | "Funciona, vive en tu PC, no la molesta" | ~done (S08 pendiente manual) |
| M002 | V1.1 | "La app que cualquiera puede usar" | próximo |
| M003 | V2.0 | "Más providers, lectura enriquecida" | planificado |
| M004 | V2.5 | "Escribí desde Skeepy" | planificado |
| M005 | V3.0 | "El ecosistema Microsoft" | planificado |
| M006 | V3.5 | "Plataforma abierta" | planificado |
| M007 | V4.0 | "Multi-plataforma" | planificado |
| M008 | V5.0 | "IA integrada" | planificado |

---

## M001 — V1.0: "Funciona, vive en tu PC, no la molesta"

Ver `.gsd/milestones/M001/M001-ROADMAP.md`

**Slices:** S01-S08 (todos completos excepto T05 de S08 — manual)

---

## M002 — V1.1: "La app que cualquiera puede usar"

**Slices:** S09-S12

Ver `.gsd/milestones/M002/M002-ROADMAP.md`

**Success criteria:**
- Cualquier usuario sin conocimiento técnico puede conectar Google Keep desde la UI
- Estado de cada provider visible en la app (activo, error, desconectado)
- Repo en GitHub con README claro + primera release pública
- Google OAuth verification iniciada

---

## M003 — V2.0: "Más providers, lectura enriquecida"

**Slices:** S13-S17

Ver `.gsd/milestones/M003/M003-ROADMAP.md`

**Success criteria:**
- Windows Sticky Notes se importa sin configuración extra
- Carpeta de archivos Markdown se puede agregar como provider
- Note detail view muestra la nota completa con todo su contenido
- El usuario puede agregar/quitar providers desde la UI sin tocar código

---

## M004 — V2.5: "Escribí desde Skeepy"

**Slices:** S18-S22

Ver `.gsd/milestones/M004/M004-ROADMAP.md`

**Success criteria:**
- El usuario puede crear y editar notas locales desde la app
- Editor inline funcional (texto plano + checklist)
- Write bidireccional con Google Keep
- Auto-update funciona: nueva versión se instala sin intervención del usuario
- Instalador firmado (SmartScreen no muestra advertencia)

---

## M005 — V3.0: "El ecosistema Microsoft"

**Slices:** S23-S25

Ver `.gsd/milestones/M005/M005-ROADMAP.md`

**Success criteria:**
- OneNote como provider de lectura y escritura
- App disponible en Microsoft Store (MSIX)
- Instalador firmado con OV cert

---

## M006 — V3.5: "Plataforma abierta"

**Slices:** S26-S31

Ver `.gsd/milestones/M006/M006-ROADMAP.md`

**Success criteria:**
- Notion como provider (lectura + escritura)
- Obsidian vault como provider local
- Plugin system: terceros pueden publicar providers como paquetes instalables
- Export a JSON/Markdown/PDF funcional
- Labels/tags se pueden gestionar desde la UI

---

## M007 — V4.0: "Multi-plataforma"

**Slices:** S32-S36

Ver `.gsd/milestones/M007/M007-ROADMAP.md`

**Success criteria:**
- App funciona en macOS (tray, autostart con Keychain, notificaciones)
- App funciona en las principales distros Linux (GNOME, KDE)
- Companion app para iOS
- Companion app para Android

---

## M008 — V5.0: "IA integrada"

**Slices:** S37-S41

Ver `.gsd/milestones/M008/M008-ROADMAP.md`

**Success criteria:**
- Búsqueda semántica local (sin enviar datos a la nube)
- Resumen automático de notas largas con LLM local
- Sugerencia de labels automática
- Graph view con backlinks entre notas
- Resolución de conflictos asistida por IA

---

## Slice index global

| Slice | Milestone | Nombre | Estado |
|-------|-----------|--------|--------|
| S01 | M001 | Core Domain | done |
| S02 | M001 | Storage Layer | done |
| S03 | M001 | Local JSON Provider | done |
| S04 | M001 | Tauri Shell + Windows Integration | done |
| S05 | M001 | Solid.js UI Core | done |
| S06 | M001 | Google Keep Provider | done |
| S07 | M001 | Polish + QA | done |
| S08 | M001 | NSIS Installer + Release | ~done |
| S09 | M002 | Keep OAuth Connect UI | pendiente |
| S10 | M002 | Provider Status Dashboard | pendiente |
| S11 | M002 | GitHub Repo + README + Release pública | pendiente |
| S12 | M002 | Google OAuth Verification | pendiente |
| S13 | M003 | Windows Sticky Notes Provider | pendiente |
| S14 | M003 | Markdown Folder Provider | pendiente |
| S15 | M003 | Note Detail View | pendiente |
| S16 | M003 | Provider Manager UI | pendiente |
| S17 | M003 | Sync robustez + error recovery UI | pendiente |
| S18 | M004 | Write Support — Local Provider | pendiente |
| S19 | M004 | Inline Editor | pendiente |
| S20 | M004 | Write Support — Google Keep | pendiente |
| S21 | M004 | Auto-Update | pendiente |
| S22 | M004 | Code Signing | pendiente |
| S23 | M005 | OneNote Provider | pendiente |
| S24 | M005 | Write Support — OneNote | pendiente |
| S25 | M005 | MSIX + Microsoft Store | pendiente |
| S26 | M006 | Notion Provider | pendiente |
| S27 | M006 | Obsidian Provider | pendiente |
| S28 | M006 | Plugin System (WASM) | pendiente |
| S29 | M006 | Export (JSON / Markdown / PDF) | pendiente |
| S30 | M006 | Labels/Tags Management UI | pendiente |
| S31 | M006 | Conflict Resolution UI | pendiente |
| S32 | M007 | macOS Port | pendiente |
| S33 | M007 | Linux Port | pendiente |
| S34 | M007 | iOS Companion | pendiente |
| S35 | M007 | Android Companion | pendiente |
| S36 | M007 | Cloud Layout Sync | pendiente |
| S37 | M008 | Semantic Search (sqlite-vec local) | pendiente |
| S38 | M008 | LLM local — Resumen + Categorización | pendiente |
| S39 | M008 | Graph View + Backlinks | pendiente |
| S40 | M008 | AI Conflict Resolution | pendiente |
| S41 | M008 | Smart Sync Scheduler | pendiente |
