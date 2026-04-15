// TypeScript types mirroring Rust domain structs.
// Keep these in sync with src-tauri/crates/core/src/note.rs

export type NoteId = string; // Uuid as string

export type NoteColor =
  | "default"
  | "red"
  | "orange"
  | "yellow"
  | "green"
  | "teal"
  | "blue"
  | "dark_blue"
  | "purple"
  | "pink"
  | "brown"
  | "gray";

// Matches Rust: #[serde(tag = "status", rename_all = "snake_case")]
export type SyncState =
  | { status: "local_only" }
  | { status: "synced"; at: string }
  | { status: "local_ahead" }
  | { status: "remote_ahead" }
  | { status: "conflict"; remote_title?: string | null; remote_updated_at: string }
  | { status: "sync_error"; message: string; retries: number };

export interface ChecklistItem {
  text: string;
  checked: boolean;
}

export type NoteContent =
  | { type: "Text"; content: string }
  | { type: "Checklist"; items: ChecklistItem[] };

export interface Label {
  id: string;
  name: string;
}

export interface Point {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface NoteLayout {
  position: Point | null;
  size: Size | null;
  visible: boolean;
  always_on_top: boolean;
  z_order: number;
}

export interface Note {
  id: NoteId;
  source_id: string;
  provider_id: string;
  title: string | null;
  content: NoteContent;
  labels: Label[];
  color: NoteColor;
  is_pinned: boolean;
  is_archived: boolean;
  is_trashed: boolean;
  created_at: string;
  updated_at: string;
  synced_at: string | null;
  sync_state: SyncState;
  layout: NoteLayout;
}

export interface NoteSearchResult {
  note: Note;
  excerpt: string | null;
  rank: number;
}

export interface AppSettings {
  sync_interval_minutes: number;
  startup_with_windows: boolean;
  show_in_tray: boolean;
  default_note_color: NoteColor;
  theme: "system" | "light" | "dark";
  enabled_providers: string[];
  telemetry_enabled: boolean;
}

export interface SyncProgressEvent {
  provider_id: string;
  status: "ok" | "error";
  notes_synced: number;
  error: string | null;
}

export type ProviderStability = "stable" | "experimental" | "deprecated";

export interface ProviderCapabilities {
  can_read: boolean;
  can_write: boolean;
  can_delete: boolean;
  supports_labels: boolean;
  supports_colors: boolean;
  supports_checklists: boolean;
  supports_incremental_sync: boolean;
  stability: ProviderStability;
}

export type ProviderStatusTag =
  | { status: "active" }
  | { status: "unauthenticated" }
  | { status: "rate_limited"; retry_after: string }
  | { status: "error"; message: string }
  | { status: "disabled" };

export interface ProviderStatusInfo {
  id: string;
  display_name: string;
  capabilities: ProviderCapabilities;
  status: ProviderStatusTag;
  last_sync_at: string | null;
  last_error: string | null;
}
