import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings,
  Note,
  NoteId,
  NoteLayout,
  NoteSearchResult,
  SyncProgressEvent,
} from "@/types/note";

// ─── Notes ────────────────────────────────────────────────────────────────────

export const notesGetAll = (): Promise<Note[]> =>
  invoke<Note[]>("notes_get_all");

export const notesSearch = (
  query: string,
  limit = 50
): Promise<NoteSearchResult[]> =>
  invoke<NoteSearchResult[]>("notes_search", { query, limit });

export const notesUpdateLayout = (
  id: NoteId,
  layout: NoteLayout
): Promise<void> =>
  invoke<void>("notes_update_layout", { id, layout });

// ─── Sync ─────────────────────────────────────────────────────────────────────

export const syncTrigger = (): Promise<void> =>
  invoke<void>("sync_trigger");

export const onSyncProgress = (
  cb: (event: SyncProgressEvent) => void
): Promise<UnlistenFn> =>
  listen<SyncProgressEvent>("sync://progress", (e) => cb(e.payload));

// ─── Settings ─────────────────────────────────────────────────────────────────

export const settingsGet = (): Promise<AppSettings> =>
  invoke<AppSettings>("settings_get");

export const settingsSet = (settings: AppSettings): Promise<void> =>
  invoke<void>("settings_set", { settings });

// ─── Keep Auth ────────────────────────────────────────────────────────────────

export interface KeepAuthInitResponse {
  auth_url: string;
  code_verifier: string;
  state: string;
  redirect_uri: string;
}

export const keepStartAuth = (redirectUri: string): Promise<KeepAuthInitResponse> =>
  invoke<KeepAuthInitResponse>("keep_start_auth", { redirectUri });

export const keepCompleteAuth = (params: {
  code: string;
  codeVerifier: string;
  redirectUri: string;
}): Promise<void> =>
  invoke<void>("keep_complete_auth", {
    code: params.code,
    codeVerifier: params.codeVerifier,
    redirectUri: params.redirectUri,
  });

export const keepRevoke = (): Promise<void> => invoke<void>("keep_revoke");

export const keepStatus = (): Promise<boolean> => invoke<boolean>("keep_status");

// ─── Keep BYO Credentials ─────────────────────────────────────────────────────

export interface KeepCredentials {
  client_id?: string;
  client_secret?: string;
}

export const keepCredentialsGet = (): Promise<KeepCredentials> =>
  invoke<KeepCredentials>("keep_credentials_get");

export const keepCredentialsSet = (
  clientId: string | null,
  clientSecret: string | null
): Promise<void> =>
  invoke<void>("keep_credentials_set", { clientId, clientSecret });

// ─── Providers ───────────────────────────────────────────────────────────────

import type { ProviderStatusInfo } from "@/types/note";

export const providersStatus = (): Promise<ProviderStatusInfo[]> =>
  invoke<ProviderStatusInfo[]>("providers_status");

export const syncProvider = (providerId: string): Promise<void> =>
  invoke<void>("sync_provider", { providerId });

// ─── Write (local notes) ──────────────────────────────────────────────────────

export type NoteContentRequest =
  | { type: "text"; content: string }
  | { type: "checklist"; items: { text: string; checked: boolean }[] };

export const noteCreate = (params: {
  title?: string | null;
  content: NoteContentRequest;
  color?: string | null;
}): Promise<Note> => invoke<Note>("note_create", params);

export const noteUpdate = (params: {
  id: NoteId;
  title?: string | null;
  content: NoteContentRequest;
  color?: string | null;
}): Promise<Note> => invoke<Note>("note_update", params);

export const noteDelete = (id: NoteId): Promise<void> =>
  invoke<void>("note_delete", { id });

// ─── Markdown Provider ────────────────────────────────────────────────────────

export const markdownGetFolder = (): Promise<string | null> =>
  invoke<string | null>("markdown_get_folder");

export const markdownSetFolder = (path: string | null): Promise<void> =>
  invoke<void>("markdown_set_folder", { path });

// ─── OneNote Auth ────────────────────────────────────────────────────────────

export interface OneNoteAuthInitResponse {
  auth_url: string;
  code_verifier: string;
  state: string;
  redirect_uri: string;
}

export const oneNoteStartAuth = (redirectUri: string): Promise<OneNoteAuthInitResponse> =>
  invoke<OneNoteAuthInitResponse>("onenote_start_auth", { redirectUri });

export const oneNoteCompleteAuth = (params: {
  code: string;
  codeVerifier: string;
  redirectUri: string;
}): Promise<void> =>
  invoke<void>("onenote_complete_auth", {
    code: params.code,
    codeVerifier: params.codeVerifier,
    redirectUri: params.redirectUri,
  });

export const oneNoteRevoke = (): Promise<void> => invoke<void>("onenote_revoke");

export const oneNoteStatus = (): Promise<boolean> => invoke<boolean>("onenote_status");

export const oneNoteCredentialsGet = (): Promise<string | null> =>
  invoke<string | null>("onenote_credentials_get");

export const oneNoteCredentialsSet = (clientId: string | null): Promise<void> =>
  invoke<void>("onenote_credentials_set", { clientId });

// ─── Notion Auth ─────────────────────────────────────────────────────────────

export interface NotionAuthInitResponse {
  auth_url: string;
  state: string;
  redirect_uri: string;
}

export const notionStartAuth = (redirectUri: string): Promise<NotionAuthInitResponse> =>
  invoke<NotionAuthInitResponse>("notion_start_auth", { redirectUri });

export const notionCompleteAuth = (params: {
  code: string;
  redirectUri: string;
}): Promise<void> =>
  invoke<void>("notion_complete_auth", {
    code: params.code,
    redirectUri: params.redirectUri,
  });

export const notionRevoke = (): Promise<void> => invoke<void>("notion_revoke");

export const notionStatus = (): Promise<boolean> => invoke<boolean>("notion_status");

export interface NotionCredentials {
  client_id?: string;
  client_secret?: string;
  parent_page_id?: string;
}

export const notionCredentialsGet = (): Promise<NotionCredentials> =>
  invoke<NotionCredentials>("notion_credentials_get");

export const notionCredentialsSet = (
  clientId: string | null,
  clientSecret: string | null,
  parentPageId: string | null
): Promise<void> =>
  invoke<void>("notion_credentials_set", { clientId, clientSecret, parentPageId });

// ─── Obsidian Provider ───────────────────────────────────────────────────────

export const obsidianGetVault = (): Promise<string | null> =>
  invoke<string | null>("obsidian_get_vault");

export const obsidianSetVault = (path: string | null): Promise<void> =>
  invoke<void>("obsidian_set_vault", { path });

// ─── Labels ───────────────────────────────────────────────────────────────────

export interface LabelInfo {
  name: string;
  providers: string[];
  note_count: number;
  is_local: boolean;
}

export const labelsGetAll = (): Promise<LabelInfo[]> =>
  invoke<LabelInfo[]>("labels_get_all");

export const labelRename = (oldName: string, newName: string): Promise<number> =>
  invoke<number>("label_rename", { oldName, newName });

export const labelDelete = (name: string): Promise<number> =>
  invoke<number>("label_delete", { name });

// ─── Export ───────────────────────────────────────────────────────────────────

export interface ExportResult {
  path: string;
  count: number;
}

export const notesExport = (
  format: "json" | "markdown",
  providerId?: string | null
): Promise<ExportResult> =>
  invoke<ExportResult>("notes_export", { format, providerId: providerId ?? null });

// ─── Semantic Search ──────────────────────────────────────────────────────────

export interface SemanticSearchResult {
  note: Note;
  score: number;
}

export const notesSearchSemantic = (
  query: string,
  limit = 10
): Promise<SemanticSearchResult[]> =>
  invoke<SemanticSearchResult[]>("notes_search_semantic", { query, limit });

export const semanticIndexRebuild = (): Promise<void> =>
  invoke<void>("semantic_index_rebuild");

// ─── Conflict Resolution ──────────────────────────────────────────────────────

export interface ConflictInfo {
  local_title: string | null;
  local_content_text: string;
  local_updated_at: string;
  remote_title: string | null;
  remote_content_text: string;
  remote_updated_at: string;
}

export const noteGetConflict = (id: string): Promise<ConflictInfo> =>
  invoke<ConflictInfo>("note_get_conflict", { id });

export const noteResolveConflict = (
  id: string,
  keep: "local" | "remote"
): Promise<Note> =>
  invoke<Note>("note_resolve_conflict", { id, keep });

// ─── Updater ─────────────────────────────────────────────────────────────────

export interface UpdateInfo {
  version: string;
  notes: string;
}

/** Checks for a newer version. Returns the update info or null if up-to-date. */
export const updaterCheck = (): Promise<UpdateInfo | null> =>
  invoke<UpdateInfo | null>("updater_check");

/** Downloads and installs the latest version. The app restarts automatically. */
export const updaterInstall = (): Promise<void> =>
  invoke<void>("updater_install");

// ─── Utility ──────────────────────────────────────────────────────────────────

export const getDataDir = (): Promise<string> => invoke<string>("get_data_dir");
