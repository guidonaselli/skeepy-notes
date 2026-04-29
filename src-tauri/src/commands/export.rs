use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use tracing::info;

use skeepy_core::NoteContent;

use crate::state::AppState;

#[derive(Debug, serde::Serialize)]
pub struct ExportResult {
    /// Absolute path to the exported file or folder.
    pub path: String,
    /// How many notes were exported.
    pub count: usize,
}

/// Export notes to the given format.
///
/// - `format`: `"json"` | `"markdown"`
/// - `provider_id`: optional filter; `None` exports all providers
///
/// Returns the path of the created file/folder.
#[tauri::command]
pub async fn notes_export(
    state: State<'_, AppState>,
    app: AppHandle,
    format: String,
    provider_id: Option<String>,
) -> Result<ExportResult, String> {
    let all_notes = state
        .notes_repo
        .find_all()
        .await
        .map_err(|e| e.to_string())?;

    let notes: Vec<_> = all_notes
        .into_iter()
        .filter(|n| n.is_visible())
        .filter(|n| {
            provider_id
                .as_deref()
                .map(|pid| n.provider_id == pid)
                .unwrap_or(true)
        })
        .collect();

    // Base output directory: <Documents>\Skeepy Export
    let docs_dir = app
        .path()
        .document_dir()
        .map_err(|e| format!("Cannot resolve Documents dir: {e}"))?;
    let base_dir = docs_dir.join("Skeepy Export");
    std::fs::create_dir_all(&base_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();

    match format.as_str() {
        "json" => {
            let file_path = base_dir.join(format!("{timestamp}_notes.json"));
            export_json(&notes, &file_path)?;
            info!(path = %file_path.display(), count = notes.len(), "Exported notes to JSON");
            Ok(ExportResult { path: file_path.to_string_lossy().into_owned(), count: notes.len() })
        }
        "markdown" => {
            let folder_path = base_dir.join(format!("{timestamp}_notes_md"));
            std::fs::create_dir_all(&folder_path).map_err(|e| e.to_string())?;
            export_markdown(&notes, &folder_path)?;
            info!(path = %folder_path.display(), count = notes.len(), "Exported notes to Markdown");
            Ok(ExportResult { path: folder_path.to_string_lossy().into_owned(), count: notes.len() })
        }
        other => Err(format!("Unknown export format: '{other}' — expected 'json' or 'markdown'")),
    }
}

// ─── JSON export ──────────────────────────────────────────────────────────────

fn export_json(notes: &[skeepy_core::Note], path: &PathBuf) -> Result<(), String> {
    let json = serde_json::to_string_pretty(notes).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

// ─── Markdown export ──────────────────────────────────────────────────────────

fn export_markdown(notes: &[skeepy_core::Note], folder: &PathBuf) -> Result<(), String> {
    for note in notes {
        let filename = safe_filename(note.title.as_deref().unwrap_or("untitled"), &note.id.to_string());
        let path = folder.join(format!("{filename}.md"));

        let content = note_to_markdown(note);
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn note_to_markdown(note: &skeepy_core::Note) -> String {
    let mut out = String::new();

    // YAML frontmatter
    out.push_str("---\n");
    if let Some(ref title) = note.title {
        out.push_str(&format!("title: \"{}\"\n", title.replace('"', "\\\"")));
    }
    out.push_str(&format!("provider: {}\n", note.provider_id));
    out.push_str(&format!("color: {:?}\n", note.color).to_lowercase());
    out.push_str(&format!("pinned: {}\n", note.is_pinned));
    out.push_str(&format!("archived: {}\n", note.is_archived));
    if !note.labels.is_empty() {
        let label_names: Vec<_> = note.labels.iter().map(|l| format!("\"{}\"", l.name)).collect();
        out.push_str(&format!("tags: [{}]\n", label_names.join(", ")));
    }
    out.push_str(&format!("created: {}\n", note.created_at.to_rfc3339()));
    out.push_str(&format!("updated: {}\n", note.updated_at.to_rfc3339()));
    out.push_str("---\n\n");

    // Content
    match &note.content {
        NoteContent::Text(text) => {
            out.push_str(text);
        }
        NoteContent::Checklist(items) => {
            for item in items {
                let check = if item.checked { "[x]" } else { "[ ]" };
                out.push_str(&format!("- {check} {}\n", item.text));
            }
        }
    }

    out
}

fn safe_filename(title: &str, fallback_id: &str) -> String {
    let sanitized: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        fallback_id[..8.min(fallback_id.len())].to_string()
    } else {
        // Truncate to 80 chars to stay within Windows path limits
        trimmed.chars().take(80).collect()
    }
}
