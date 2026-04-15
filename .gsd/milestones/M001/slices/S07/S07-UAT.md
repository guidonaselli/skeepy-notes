# S07: UAT — User Acceptance Test Checklist

All items must pass before S08 (installer).

## Performance Criteria

- [ ] App starts in < 1 second (from tray click to window visible)
- [ ] FTS5 search responds in < 50ms for 1000 notes (measure via `notes_search` IPC timing)
- [ ] Idle RAM < 50MB (Task Manager → Working Set after 5 min idle)
- [ ] Idle CPU < 0.5% (avg over 1 min while app is in tray, no sync running)

## Core Behavior

- [ ] App appears in system tray after launch
- [ ] Window closes → hides to tray (process still running)
- [ ] Tray "Mostrar Skeepy" → window shows and gets focus
- [ ] Tray "Salir" → process exits cleanly
- [ ] Autostart with Windows is set in HKCU on first run
  - Verify: `reg query HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run /v Skeepy`

## Notes Display

- [ ] Local notes.json is loaded and displayed as cards
- [ ] Card colors match the configured color in notes.json
- [ ] Pinned notes appear first
- [ ] Archived/trashed notes are NOT shown in the main grid

## Search

- [ ] Typing in search bar shows FTS5 results within 200ms (debounce)
- [ ] Clearing the search bar restores the full note grid
- [ ] Search works for partial words (FTS5 porter tokenizer)

## Layout Persistence

- [ ] Dragging a note card saves its position (visible after restart)
- [ ] Position persists in SQLite `note_layouts` table

## Sync

- [ ] `sync_trigger` IPC runs LocalProvider sync successfully
- [ ] `sync://progress` event is emitted with `status: "ok"` after sync
- [ ] If notes.json is missing → no error, empty grid (not crash)
- [ ] If notes.json is invalid JSON → error logged, no crash

## Settings

- [ ] `settings_get` returns defaults on first run
- [ ] Changing `sync_interval_minutes` persists across restart
- [ ] Theme toggle updates the setting (actual theme not required for V1)

## Google Keep (Optional — requires OAuth setup)

- [ ] `keep_start_auth` returns a valid Google auth URL
- [ ] After completing auth flow, tokens are in Windows Credential Manager
  - Verify: Windows Credential Manager → "skeepy-notes" entry exists
- [ ] Keep notes appear in the grid mixed with local notes
- [ ] `keep_revoke` removes the entry from Credential Manager
- [ ] Keep provider shows `ProviderStability::Experimental` in capabilities

## Stability

- [ ] App runs for 8 hours without memory growth (< 10MB increase)
- [ ] Network offline → sync fails gracefully (no crash), error logged
- [ ] App restart after 8h still loads all notes correctly

## Error Paths

- [ ] Invalid notes.json → `ProviderError::Api` logged, app continues
- [ ] SQLite DB locked → `StorageError::Database` logged, IPC returns error string (no panic)
- [ ] Keep API returns 401 → `ProviderError::AuthRequired`, user sees "reconnect" prompt (V1: logged only)
