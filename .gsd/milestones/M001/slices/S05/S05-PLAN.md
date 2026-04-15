# S05: Solid.js UI — Core

**Goal:** Frontend funcional que muestra notas como cards, permite buscar con FTS5, filtra por provider, persiste el layout (posición + tamaño) de cada card entre reinicios.
**Demo:** `cargo tauri dev` abre la ventana con notas cargadas desde SQLite, búsqueda responde en < 50ms, arrastrar una nota persiste su posición al reiniciar.

## Must-Haves

- Solid.js + Vite wired to Tauri (`devUrl`, `frontendDist`)
- `tauri.service.ts` — typed wrapper para `invoke()` y `listen()`
- `notes.store.ts` — `Map<string, Note>` reactivo, se carga on-mount
- `sync.store.ts` — estado por provider (ok/error/syncing)
- Componente `NoteCard` — title, content preview, color, pinned badge, provider badge
- Componente `NoteGrid` — renderiza todas las notas visibles
- Componente `SearchBar` — FTS5 search con debounce 200ms
- Componente `ProviderBadge` — "local" | "keep" con color
- Layout persistente: drag-to-move con `onMouseUp` → `notes_update_layout` IPC
- Componente `Settings` — panel con sync_interval, startup_with_windows, theme toggle
- Trigger sync on mount, escuchar `sync://progress` event para refrescar

## Out of Scope

- Keep OAuth flow (S06)
- Resize handles (nice-to-have V2)
- Animations / transitions

## Tasks

- [ ] **T01: Vite + Solid.js setup**
  - `package.json`, `vite.config.ts`, `tsconfig.json`
  - Update `tauri.conf.json` devUrl a `http://localhost:1420`
  - `src/main.tsx` entry point

- [ ] **T02: IPC service layer**
  - `src/services/tauri.service.ts` — typed `invoke<T>()` wrappers
  - `src/types/note.ts` — tipos TypeScript espejando los structs Rust

- [ ] **T03: Stores**
  - `src/stores/notes.store.ts` — createStore + load + search
  - `src/stores/sync.store.ts` — estado por provider + listener

- [ ] **T04: Core components**
  - `src/components/NoteCard.tsx`
  - `src/components/NoteGrid.tsx`
  - `src/components/SearchBar.tsx`
  - `src/components/ProviderBadge.tsx`

- [ ] **T05: Layout persistence**
  - drag handler en NoteCard
  - `notes_update_layout` IPC call on mouseup

- [ ] **T06: Settings panel**
  - `src/components/Settings.tsx`

- [ ] **T07: App root**
  - `src/App.tsx` — compone todo, trigger sync on mount

## Files Likely Touched

- `package.json`
- `vite.config.ts`
- `tsconfig.json`
- `src/main.tsx`
- `src/App.tsx`
- `src/types/note.ts`
- `src/services/tauri.service.ts`
- `src/stores/notes.store.ts`
- `src/stores/sync.store.ts`
- `src/components/NoteCard.tsx`
- `src/components/NoteGrid.tsx`
- `src/components/SearchBar.tsx`
- `src/components/ProviderBadge.tsx`
- `src/components/Settings.tsx`
- `src/styles/global.css`
