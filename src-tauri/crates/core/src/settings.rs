use serde::{Deserialize, Serialize};
use crate::note::{NoteColor, ProviderId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// How often to sync with remote providers, in minutes. Default: 15.
    pub sync_interval_minutes: u32,
    /// Register the app in HKCU autostart on Windows.
    pub startup_with_windows: bool,
    /// Keep the app in the system tray after closing the main window.
    pub show_in_tray: bool,
    /// Default color for newly created notes.
    pub default_note_color: NoteColor,
    pub theme: Theme,
    /// List of provider IDs that are enabled. Order is display order.
    pub enabled_providers: Vec<ProviderId>,
    /// Telemetry is OFF by default — must be explicitly opted in.
    pub telemetry_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            sync_interval_minutes: 15,
            startup_with_windows: true,
            show_in_tray: true,
            default_note_color: NoteColor::Default,
            theme: Theme::System,
            enabled_providers: vec!["local".to_string()],
            telemetry_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe() {
        let s = AppSettings::default();
        assert_eq!(s.sync_interval_minutes, 15);
        assert!(!s.telemetry_enabled);
        assert!(s.startup_with_windows);
        assert_eq!(s.enabled_providers, vec!["local"]);
    }
}
