mod commands;
mod semantic;
mod smart_sync;
mod state;

use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};
use tokio::sync::RwLock;
use tracing::info;

use skeepy_core::{NoteProvider, NoteRepository, SettingsRepository};
use skeepy_provider_local::LocalProvider;
use skeepy_provider_markdown::MarkdownProvider;
use skeepy_provider_notion::provider::NotionProvider;
use skeepy_provider_notion::TokenStorage as NotionTokenStorage;
use skeepy_provider_obsidian::ObsidianProvider;
use skeepy_provider_onenote::provider::OneNoteProvider;
use skeepy_provider_onenote::TokenStorage as OneNoteTokenStorage;
#[cfg(target_os = "windows")]
use skeepy_provider_sticky_notes::StickyNotesProvider;
use skeepy_storage::{Database, SqliteNoteRepository, SqliteSettingsRepository};

use crate::commands::{conflict, export, graph, keep, labels, markdown, notes, notion, obsidian, onenote, providers, search, settings, sync, updater, window, write};
use crate::state::AppState;

pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            // The MacosLauncher variant is ignored on Windows — HKCU registry is used.
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_oauth::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&data_dir)?;

            let db_path = data_dir.join("skeepy.db");
            info!(path = %db_path.display(), "Opening database");

            let db = Arc::new(Database::open(&db_path).map_err(|e| {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
                    as Box<dyn std::error::Error>
            })?);

            let notes_repo = Arc::new(SqliteNoteRepository::new(Arc::clone(&db)));
            let settings_repo = Arc::new(SqliteSettingsRepository::new(Arc::clone(&db)));
            // Clone for startup window creation (before notes_repo is moved into AppState).
            let notes_repo_startup = Arc::clone(&notes_repo);

            // LocalProvider reads from <data_dir>/notes.json — no error if missing.
            let local_notes_path = data_dir.join("notes.json");
            info!(path = %local_notes_path.display(), "LocalProvider path");

            // Markdown provider — folder restored from settings if previously configured.
            let markdown_provider = MarkdownProvider::new();
            let markdown_folder = markdown_provider.folder_handle();

            // Restore persisted markdown folder path.
            if let Ok(Some(saved_path)) = tauri::async_runtime::block_on(
                settings_repo.get_raw("markdown_folder_path")
            ) {
                if !saved_path.is_empty() {
                    if let Ok(mut lock) = markdown_folder.write() {
                        *lock = Some(std::path::PathBuf::from(saved_path));
                    }
                }
            }

            // Obsidian provider — vault path restored from settings.
            let obsidian_provider = ObsidianProvider::new();
            let obsidian_vault = obsidian_provider.vault_handle();

            if let Ok(Some(saved_vault)) = tauri::async_runtime::block_on(
                settings_repo.get_raw("obsidian_vault_path")
            ) {
                if !saved_vault.is_empty() {
                    if let Ok(mut lock) = obsidian_vault.write() {
                        *lock = Some(std::path::PathBuf::from(saved_vault));
                    }
                }
            }

            // Auto-register Notion if tokens already exist.
            let notion_provider: Option<Box<dyn NoteProvider>> =
                if let Ok(Some(_)) = NotionTokenStorage::load() {
                    let client_id = tauri::async_runtime::block_on(settings_repo.get_raw("notion_client_id"))
                        .ok().flatten()
                        .filter(|s: &String| !s.is_empty())
                        .unwrap_or_else(|| option_env!("NOTION_CLIENT_ID").unwrap_or("").to_string());
                    let client_secret = tauri::async_runtime::block_on(settings_repo.get_raw("notion_client_secret"))
                        .ok().flatten()
                        .filter(|s: &String| !s.is_empty())
                        .unwrap_or_else(|| option_env!("NOTION_CLIENT_SECRET").unwrap_or("").to_string());
                    let parent_id = tauri::async_runtime::block_on(settings_repo.get_raw("notion_parent_page_id"))
                        .ok().flatten()
                        .filter(|s: &String| !s.is_empty());
                    if client_id.is_empty() { None }
                    else { Some(Box::new(NotionProvider::new(client_id, client_secret, parent_id))) }
                } else { None };

            // Auto-register OneNote if tokens already exist from a previous session.
            let onenote_provider: Option<Box<dyn NoteProvider>> =
                if let Ok(Some(_)) = OneNoteTokenStorage::load() {
                    let client_id = tauri::async_runtime::block_on(
                        settings_repo.get_raw("onenote_client_id")
                    )
                    .ok()
                    .flatten()
                    .filter(|s: &String| !s.is_empty())
                    .unwrap_or_else(|| {
                        option_env!("AZURE_CLIENT_ID").unwrap_or("").to_string()
                    });
                    if client_id.is_empty() {
                        None
                    } else {
                        Some(Box::new(OneNoteProvider::new(client_id)))
                    }
                } else {
                    None
                };

            let mut providers: Vec<Box<dyn NoteProvider>> = vec![
                Box::new(LocalProvider::new(local_notes_path)),
                Box::new(markdown_provider),
                Box::new(obsidian_provider),
            ];
            #[cfg(target_os = "windows")]
            providers.insert(1, Box::new(StickyNotesProvider::new()));
            if let Some(p) = notion_provider {
                providers.push(p);
            }
            if let Some(p) = onenote_provider {
                providers.push(p);
            }

            let state = AppState {
                db,
                notes_repo: notes_repo as Arc<dyn skeepy_core::NoteRepository>,
                settings_repo: settings_repo as Arc<dyn skeepy_core::SettingsRepository>,
                providers: Arc::new(RwLock::new(providers)),
                markdown_folder,
                obsidian_vault,
            };

            app.manage(state);

            // ── Open sticky note windows for all previously visible notes ──────
            let startup_notes = tauri::async_runtime::block_on(
                notes_repo_startup.find_all()
            ).unwrap_or_default();

            let visible_notes: Vec<_> = startup_notes
                .into_iter()
                .filter(|n| n.layout.visible && !n.is_trashed)
                .collect();

            for note in &visible_notes {
                let label = format!("note-{}", note.id);
                let pos = note.layout.position.unwrap_or(skeepy_core::Point { x: 120.0, y: 120.0 });
                let size = note.layout.size.unwrap_or(skeepy_core::Size { width: 280.0, height: 220.0 });
                let _ = tauri::WebviewWindowBuilder::new(
                    app,
                    &label,
                    tauri::WebviewUrl::App(format!("index.html?note={}", note.id).into()),
                )
                .title("")
                .inner_size(size.width as f64, size.height as f64)
                .position(pos.x as f64, pos.y as f64)
                .decorations(false)
                .resizable(true)
                .skip_taskbar(true)
                .always_on_top(note.layout.always_on_top)
                .build();
            }

            // Show manager only on first run (no notes yet); otherwise it stays hidden.
            if let Some(main_win) = app.get_webview_window("main") {
                if visible_notes.is_empty() {
                    let _ = main_win.show();
                    let _ = main_win.set_focus();
                }
            }

            setup_tray(app)?;
            setup_autostart(app);

            // Record that the app was opened (used by smart sync scheduler).
            if let Some(s) = app.try_state::<AppState>() {
                smart_sync::record_app_open(&s);
                // Kick off semantic index build for any un-indexed notes.
                semantic::indexer::index_in_background(s.db.clone());
            }

            start_periodic_sync(app.handle().clone());
            start_update_check(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label().to_string();
                if label == "main" {
                    // Manager: hide instead of close so the process stays alive.
                    api.prevent_close();
                    let _ = window.hide();
                } else if label.starts_with("note-") {
                    // Sticky note window closed without going through the in-window
                    // close button (e.g. Alt+F4, taskbar close). Persist visible=false.
                    let id_str = label.trim_start_matches("note-").to_string();
                    let app = window.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        if let Ok(id) = id_str.parse::<uuid::Uuid>() {
                            if let Some(state) = app.try_state::<AppState>() {
                                if let Ok(Some(note)) = state.notes_repo.find_by_id(&id).await {
                                    let mut layout = note.layout;
                                    layout.visible = false;
                                    let _ = state.notes_repo.update_layout(&id, &layout).await;
                                    // Notify the manager so it refreshes the note's visual state.
                                    let _ = app.emit("note://layout-changed", serde_json::json!({ "id": id.to_string() }));
                                }
                            }
                        }
                    });
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            notes::notes_get_all,
            notes::notes_search,
            notes::notes_update_layout,
            sync::sync_trigger,
            settings::settings_get,
            settings::settings_set,
            keep::keep_start_auth,
            keep::keep_complete_auth,
            keep::keep_revoke,
            keep::keep_status,
            keep::keep_credentials_get,
            keep::keep_credentials_set,
            providers::providers_status,
            providers::sync_provider,
            markdown::markdown_get_folder,
            markdown::markdown_set_folder,
            notion::notion_start_auth,
            notion::notion_complete_auth,
            notion::notion_revoke,
            notion::notion_status,
            notion::notion_credentials_get,
            notion::notion_credentials_set,
            obsidian::obsidian_get_vault,
            obsidian::obsidian_set_vault,
            onenote::onenote_start_auth,
            onenote::onenote_complete_auth,
            onenote::onenote_revoke,
            onenote::onenote_status,
            onenote::onenote_credentials_get,
            onenote::onenote_credentials_set,
            write::note_create,
            write::note_update,
            write::note_update_color,
            write::note_delete,
            conflict::note_get_conflict,
            conflict::note_resolve_conflict,
            search::notes_search_semantic,
            search::semantic_index_rebuild,
            graph::notes_get_graph,
            export::notes_export,
            labels::labels_get_all,
            labels::label_rename,
            labels::label_delete,
            updater::updater_check,
            updater::updater_install,
            window::note_window_show,
            window::note_window_close,
            notes::note_get,
            get_data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Skeepy");
}

// ─── Tray ─────────────────────────────────────────────────────────────────────

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let new_note = MenuItem::with_id(app, "new_note", "Nueva nota", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Mostrar manager", true, None::<&str>)?;
    let sync_now = MenuItem::with_id(app, "sync", "Sincronizar ahora", true, None::<&str>)?;
    let check_updates = MenuItem::with_id(app, "check_updates", "Buscar actualizaciones", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&new_note, &show, &sync_now, &check_updates, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::InvalidIcon(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No default icon configured",
        )))?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("Skeepy Notes")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "new_note" => {
                // Show the manager and ask it to open the create modal immediately.
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
                let _ = app.emit("note://create-requested", ());
            }
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "sync" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Err(e) = sync::run_sync(&state, &app).await {
                            tracing::error!(error = %e, "Tray sync failed");
                        }
                    }
                });
            }
            "check_updates" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    use tauri_plugin_updater::UpdaterExt;
                    match app.updater() {
                        Ok(updater) => match updater.check().await {
                            Ok(Some(update)) => {
                                let _ = app.emit("update://available", serde_json::json!({
                                    "version": update.version,
                                    "notes": update.body.unwrap_or_default(),
                                }));
                            }
                            Ok(None) => tracing::info!("App is up to date"),
                            Err(e) => tracing::warn!(error = %e, "Update check failed"),
                        },
                        Err(e) => tracing::warn!(error = %e, "Updater unavailable"),
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

// ─── Autostart ────────────────────────────────────────────────────────────────

fn setup_autostart(app: &tauri::App) {
    use tauri_plugin_autostart::ManagerExt;

    match app.autolaunch().is_enabled() {
        Ok(true) => {
            info!("Autostart already enabled");
        }
        Ok(false) => {
            // First run: opt the user in by default (they can turn it off in Settings).
            match app.autolaunch().enable() {
                Ok(_) => info!("Autostart enabled on first run"),
                Err(e) => tracing::warn!(error = %e, "Failed to enable autostart"),
            }
        }
        Err(e) => tracing::warn!(error = %e, "Failed to query autostart state"),
    }
}

// ─── Periodic sync ────────────────────────────────────────────────────────────

fn start_periodic_sync(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let interval_minutes = get_sync_interval(&app).await;
            let fallback_secs = interval_minutes * 60;

            // Use smart scheduler if we have usage history, else fall back to fixed interval.
            let sleep_secs = if let Some(state) = app.try_state::<AppState>() {
                smart_sync::seconds_until_next_smart_sync(&state, fallback_secs)
            } else {
                fallback_secs
            };

            tokio::time::sleep(tokio::time::Duration::from_secs(sleep_secs)).await;

            if let Some(state) = app.try_state::<AppState>() {
                smart_sync::record_app_open(&state);
                if let Err(e) = sync::run_sync(&state, &app).await {
                    tracing::error!(error = %e, "Periodic sync failed");
                }
                // Re-index any notes added by the sync.
                semantic::indexer::index_in_background(state.db.clone());
            }
        }
    });
}

// ─── Background update check ──────────────────────────────────────────────────

fn start_update_check(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Wait 30 s after startup to avoid slowing down the initial load.
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

        use tauri_plugin_updater::UpdaterExt;
        match app.updater() {
            Ok(updater) => match updater.check().await {
                Ok(Some(update)) => {
                    let _ = app.emit("update://available", serde_json::json!({
                        "version": update.version,
                        "notes": update.body.unwrap_or_default(),
                    }));
                }
                Ok(None) => tracing::debug!("No update available"),
                Err(e) => tracing::warn!(error = %e, "Update check failed"),
            },
            Err(e) => tracing::debug!(error = %e, "Updater not configured"),
        }
    });
}

async fn get_sync_interval(app: &tauri::AppHandle) -> u64 {
    const DEFAULT_MINUTES: u64 = 15;
    let Some(state) = app.try_state::<AppState>() else { return DEFAULT_MINUTES };
    match state.settings_repo.get_raw("app_settings").await {
        Ok(Some(json)) => serde_json::from_str::<skeepy_core::AppSettings>(&json)
            .map(|s| s.sync_interval_minutes as u64)
            .unwrap_or(DEFAULT_MINUTES),
        _ => DEFAULT_MINUTES,
    }
}

// ─── Utility commands ─────────────────────────────────────────────────────────

#[tauri::command]
fn get_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

// ─── Tracing ──────────────────────────────────────────────────────────────────

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "skeepy=info,skeepy_app_lib=info".into()),
        )
        .init();
}
