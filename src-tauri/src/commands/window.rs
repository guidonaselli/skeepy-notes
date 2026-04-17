use tauri::{AppHandle, Emitter, Manager, State, WebviewWindowBuilder, WebviewUrl};

use skeepy_core::{NoteId, Point, Size};

use crate::state::AppState;

/// Open (or focus) the sticky note window for a given note ID.
/// Also marks layout.visible = true in the database.
#[tauri::command]
pub async fn note_window_show(
    app: AppHandle,
    state: State<'_, AppState>,
    id: NoteId,
) -> Result<(), String> {
    let label = format!("note-{id}");

    if let Some(existing) = app.get_webview_window(&label) {
        existing.show().map_err(|e| e.to_string())?;
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let note = state
        .notes_repo
        .find_by_id(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Note {id} not found"))?;

    let pos = note.layout.position.unwrap_or(Point { x: 120.0, y: 120.0 });
    let size = note.layout.size.unwrap_or(Size { width: 280.0, height: 220.0 });

    WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::App(format!("index.html?note={id}").into()),
    )
    .title("")
    .inner_size(size.width as f64, size.height as f64)
    .position(pos.x as f64, pos.y as f64)
    .decorations(false)
    .resizable(true)
    .skip_taskbar(true)
    .always_on_top(note.layout.always_on_top)
    .build()
    .map_err(|e| e.to_string())?;

    // Persist visible = true
    let mut layout = note.layout;
    layout.visible = true;
    state
        .notes_repo
        .update_layout(&id, &layout)
        .await
        .map_err(|e| e.to_string())?;

    // Notify the manager window so it can refresh the note's visual state.
    let _ = app.emit("note://layout-changed", serde_json::json!({ "id": id.to_string() }));

    Ok(())
}

/// Close the sticky note window for a given note ID (does NOT change layout.visible;
/// the frontend calls notes_update_layout before invoking this, or the close event
/// in lib.rs handles persistence).
#[tauri::command]
pub fn note_window_close(app: AppHandle, id: NoteId) -> Result<(), String> {
    let label = format!("note-{id}");
    if let Some(w) = app.get_webview_window(&label) {
        w.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
