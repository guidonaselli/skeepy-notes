import { createStore } from "solid-js/store";
import type { SyncProgressEvent } from "@/types/note";
import { onSyncProgress, syncTrigger } from "@/services/tauri.service";
import { loadNotes } from "./notes.store";

export type ProviderSyncStatus = "idle" | "syncing" | "ok" | "error";

interface ProviderSyncState {
  status: ProviderSyncStatus;
  lastSyncAt: string | null;
  lastError: string | null;
  notesSynced: number;
}

interface SyncState {
  providers: Record<string, ProviderSyncState>;
  isSyncing: boolean;
}

const [state, setState] = createStore<SyncState>({
  providers: {},
  isSyncing: false,
});

export const syncStore = {
  get isSyncing() { return state.isSyncing; },
  getProvider: (id: string): ProviderSyncState =>
    state.providers[id] ?? { status: "idle", lastSyncAt: null, lastError: null, notesSynced: 0 },
};

function applyProgress(event: SyncProgressEvent): void {
  setState("providers", event.provider_id, {
    status: event.status as ProviderSyncStatus,
    lastSyncAt: event.status === "ok" ? new Date().toISOString() : (state.providers[event.provider_id]?.lastSyncAt ?? null),
    lastError: event.error ?? null,
    notesSynced: event.notes_synced,
  });

  // Refresh notes list when sync brings in changes
  if (event.status === "ok" && event.notes_synced > 0) {
    void loadNotes();
  }
}

/** Call once at app startup to wire up the sync event listener. */
export async function initSyncListener(): Promise<void> {
  await onSyncProgress(applyProgress);
}

export async function triggerSync(): Promise<void> {
  if (state.isSyncing) return;
  setState("isSyncing", true);

  // Mark all known providers as syncing
  for (const id of Object.keys(state.providers)) {
    setState("providers", id, "status", "syncing");
  }

  try {
    await syncTrigger();
  } catch (e) {
    console.error("Sync trigger failed:", e);
  } finally {
    setState("isSyncing", false);
  }
}
