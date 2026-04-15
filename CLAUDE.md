# Skeepy — Agent Instructions

## PRIMER PASO OBLIGATORIO

**Antes de hacer cualquier cosa, leé `.gsd/STATE.md`.**
Ese archivo te dice exactamente dónde estamos, qué es lo próximo y si hay trabajo interrumpido.

Si hay un `continue.md` en el slice activo → leelo primero y retomá desde ahí.

---

## Qué es este proyecto

Skeepy es una app de escritorio open source para Windows. Agregador/visor de notas local con arquitectura por providers. Stack: Rust + Tauri 2.x + Solid.js + SQLite FTS5.

**No es un cliente de Google Keep.** Keep es un provider opcional.

---

## Cómo navegar el planning

```
.gsd/STATE.md                              ← SIEMPRE PRIMERO (estado actual + próxima acción)
.gsd/DECISIONS.md                          ← Leer antes de cualquier decisión técnica
.gsd/milestones/M001/M001-ROADMAP.md       ← Qué slices existen y cuáles están hechos
.gsd/milestones/M001/M001-CONTEXT.md       ← Stack, domain model, decisiones de arquitectura
.gsd/milestones/M001/M001-RESEARCH.md      ← Pitfalls, don't hand-roll, APIs verificadas
.gsd/milestones/M001/slices/S##/S##-PLAN.md ← Tareas del slice activo
```

**Flujo de una sesión:**
1. Leer `STATE.md` → saber dónde estamos
2. Leer `DECISIONS.md` → respetar decisiones existentes
3. Leer el `S##-PLAN.md` del slice activo → encontrar próxima tarea
4. Hacer la tarea → verificar must-haves → escribir summary
5. Marcar done en el plan → actualizar `STATE.md`

---

## Reglas de trabajo

- Un task = una context-window. Si no entra, dividir.
- Verificar must-haves antes de marcar una tarea como done.
- Si se toma una decisión técnica → agregar fila a `DECISIONS.md` (append-only).
- Si el contexto se llena antes de terminar un task → escribir `continue.md` en el slice activo.
- Nunca agregar dependencias sin justificación. Ver `M001-RESEARCH.md` sección "Don't Hand-Roll".

---

## Stack quick reference

| Qué | Cómo |
|-----|------|
| Backend/core | Rust, Cargo workspace, crates en `src-tauri/crates/` |
| Desktop shell | Tauri 2.x (`src-tauri/src/main.rs`) |
| UI | Solid.js (`src/`) |
| Storage | SQLite + FTS5 via `sqlx` |
| Tokens OAuth | `keyring` crate (DPAPI) — NUNCA en SQLite |
| Autostart | `tauri-plugin-autostart` (HKCU, sin admin) |
| Google Keep | `notes.googleapis.com/v1`, scope `keep.readonly` |
