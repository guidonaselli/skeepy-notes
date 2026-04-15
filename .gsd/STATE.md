# GSD State

**Active Milestone:** M007 + M008 (en progreso en paralelo)
**Active Slice:** S32-S34 (M007 cross-platform), S37 (semantic search TF-IDF), S41 (smart sync)
**Active Task:** M007 build configs done. M008 S37 infraestructura done. S41 smart scheduler done.
**Phase:** M006 completo. M007 S32-S34 done. M008 S37 infra + S41 done.

## Completed Slices
- [x] **S01: Core Domain** — 21 tests passing.
- [x] **S02: Storage Layer** — 13 tests passing.
- [x] **S03: Local JSON Provider** — 9 tests passing.
- [x] **S04: Tauri Shell** — 0 warnings. Tray, autostart, IPC commands.
- [x] **S05: Solid.js UI** — Vite builds. NoteCard, NoteGrid, SearchBar, Settings, stores.
- [x] **S06: Google Keep Provider** — KeepProvider OAuth2 PKCE, keyring/DPAPI.
- [x] **S07: Polish + QA** — Periodic sync, get_data_dir, Keep Connect UI.
- [x] **S08: NSIS Installer** — `cargo tauri build` corrió exitosamente.
- [x] **S09: Keep OAuth Connect UI** — tauri-plugin-oauth, handleKeepConnect(), CSRF.
- [x] **S10: Provider Status Dashboard** — providers_status + sync_provider IPC. ProviderStatusPanel.
- [x] **S13: Windows Sticky Notes Provider** — skeepy-provider-sticky-notes. plum.sqlite, Soft XAML strip.
- [x] **S14: Markdown Folder Provider** — skeepy-provider-markdown. Frontmatter, SHA-256, IPC.
- [x] **S15: Note Detail View** — NoteDetailPanel + expand button.
- [x] **S16: Provider Manager UI** — errores inline, stability badge.
- [x] **S17: Sync Robustez + Error Recovery UI** — Backoff exponencial, error banners.
- [x] **S18: Write Support Local** — note_create / note_update / note_delete IPC, CreateNoteModal, Ctrl+N.
- [x] **S19: Inline Editor** — NoteDetailPanel modo edición, Ctrl+S, Escape, delete.
- [x] **S20: Write Support Google Keep** — Keep API POST/DELETE, can_write/can_delete.
- [x] **S21: Auto-Update** — tauri-plugin-updater implementado. Background check 30s post-startup. Emite update://available. IPC: updater_check / updater_install. Tray item. FALTA: generar keys + agregar secrets en GitHub + reemplazar pubkey placeholder en tauri.conf.json.
- [~] **S22: Code Signing** — Manual. Requiere certificado OV.
- [~] **S11: GitHub Repo + Release** — Manual: crear repo, push + tag v0.1.0.
- [ ] **S12: Google OAuth Verification** — proceso burocrático.
- [x] **S23: OneNote Provider — Lectura** — skeepy-provider-onenote. Microsoft Graph API, PKCE, keyring. IPC: onenote_*. Settings UI.
- [x] **S24: Write Support OneNote** — update_note trait + OneNote PATCH API + write.rs routing. NoteDetailPanel isEditable=onenote.
- [~] **S25: MSIX + Microsoft Store** — Requiere S22 (code signing). Manual.
- [x] **S26: Notion Provider** — skeepy-provider-notion. OAuth2 (no PKCE, Basic auth). Block→text conversion. CRUD completo. IPC + Settings UI.
- [x] **S27: Obsidian Provider** — skeepy-provider-obsidian. Recursive vault walk, [[backlinks]] strip, #inline tags, Obsidian frontmatter (created/updated/aliases).
- [ ] **S28: Plugin System (WASM)** — muy alto riesgo, WIT interface design pendiente.
- [x] **S29: Export** — notes_export IPC. JSON + Markdown formats. UI en Settings.
- [x] **S30: Labels/Tags Management UI** — labels_get_all, label_rename, label_delete. LabelsPanel.tsx. Label filter bar en App.tsx.
- [x] **S31: Conflict Resolution UI** — SyncState::Conflict con datos del remote. ConflictPanel.tsx. note_get_conflict / note_resolve_conflict IPC. LocalAhead antes del push.

## Recent Decisions
- D001: Stack = Rust + Tauri 2.x
- D005: Provider model = trait-based (NoteProvider)
- D006: Sync V1 = pull-only / read-only
- D008: Google Keep API = notes.googleapis.com/v1
- D010: V1 scope = local provider + Keep read-only + FTS5 + layout persistente
- D-NEW-001: UpdateNoteRequest agregado al NoteProvider trait (default = NotSupported) para OneNote/Notion write support.
- D-NEW-002: Notion requiere client_secret para desktop apps — BYO credentials model (igual que Keep opcional).
- D-NEW-003: isEditable() en NoteDetailPanel cubre local, onenote, notion.

## Blockers
- None

## Next Action

**Listo para publicar.** Único paso pendiente:

```
pwsh scripts\setup-release.ps1 -GithubUser TU_USUARIO
```

Ese script:
1. Genera las keys minisign
2. Parchea tauri.conf.json con pubkey + endpoint URL reales
3. Muestra las keys para GitHub Secrets
4. Opcionalmente crea el repo en GitHub

Después de eso:
```
git add -A && git commit -m "chore: configure release signing"
git push -u origin main
git tag v0.1.0 && git push origin v0.1.0
```

El CI genera el NSIS installer firmado + `latest.json` y publica el release.

## Completed M007 Slices
- [x] **S32: Cross-platform build** — tauri.conf.json macOS+Linux, skeepy-provider-sticky-notes Windows-only dep
- [x] **S33: macOS** — Info.plist LSUIElement, iconAsTemplate, minimumSystemVersion 10.15
- [x] **S34: GitHub Actions matrix** — Windows/macOS x64/macOS ARM/Linux CI+release matrix
- [x] **NoteCard sync badge** — ⚡ conflict, ↑ local_ahead, ! sync_error indicators

## Completed M008 Slices
- [x] **S41: Smart Sync Scheduler** — usage_events SQLite table, histogram 168-slot weekday×hour, predicts next peak
- [x] **S37 infra: Semantic Search (TF-IDF)** — migration 004_embeddings, tfidf.rs vectorizer, indexer.rs background indexing + cosine search, notes_search_semantic IPC, SearchBar FTS↔semantic toggle

## Post-V1 Backlog

- S28: Plugin System (WASM) — wasmtime + Component Model
- S38: LLM Local — resumen + categorización (llamafile)
- S39: Graph View + Backlinks (vis-network o d3-force)
- S40: AI Conflict Resolution
- S37 upgrade: reemplazar TF-IDF por ONNX (nomic-embed-text / all-MiniLM)
