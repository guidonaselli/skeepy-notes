import { createSignal } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import type { Note, NoteId, NoteSearchResult } from "@/types/note";
import {
  notesGetAll,
  notesSearch,
  notesSearchSemantic,
  notesUpdateLayout,
  onNoteLayoutChanged,
  type SemanticSearchResult,
} from "@/services/tauri.service";
import type { NoteLayout } from "@/types/note";

// ─── State ────────────────────────────────────────────────────────────────────

interface NotesState {
  /** All notes keyed by id. */
  byId: Record<NoteId, Note>;
  /** Ordered list of visible (non-trashed, non-archived) note ids. */
  visibleIds: NoteId[];
  /** FTS5 search results (empty when not searching or in semantic mode). */
  searchResults: NoteSearchResult[];
  /** Semantic search results (empty unless mode is "semantic"). */
  semanticResults: SemanticSearchResult[];
  loading: boolean;
  error: string | null;
}

const [state, setState] = createStore<NotesState>({
  byId: {},
  visibleIds: [],
  searchResults: [],
  semanticResults: [],
  loading: false,
  error: null,
});

const [searchQuery, setSearchQuery] = createSignal("");
const [searchMode, setSearchMode] = createSignal<"fts" | "semantic">("fts");
export { searchMode, setSearchMode };

// ─── Derived ─────────────────────────────────────────────────────────────────

export const notesStore = {
  get loading() { return state.loading; },
  get error() { return state.error; },
  get query() { return searchQuery(); },
  get mode() { return searchMode(); },
  get visibleNotes(): Note[] {
    return state.visibleIds.map((id) => state.byId[id]).filter(Boolean);
  },
  get searchResults() { return state.searchResults; },
  get semanticResults() { return state.semanticResults; },
  get isSearching() { return searchQuery().trim().length > 0; },
};

// ─── Actions ──────────────────────────────────────────────────────────────────

export async function loadNotes(): Promise<void> {
  setState("loading", true);
  setState("error", null);
  try {
    const notes = await notesGetAll();
    const byId: Record<NoteId, Note> = {};
    for (const n of notes) { byId[n.id] = n; }

    const visibleIds = notes
      .filter((n) => !n.is_trashed && !n.is_archived)
      .sort((a, b) => {
        // Pinned notes first, then by updated_at desc
        if (a.is_pinned !== b.is_pinned) return a.is_pinned ? -1 : 1;
        return b.updated_at.localeCompare(a.updated_at);
      })
      .map((n) => n.id);

    setState(reconcile({ byId, visibleIds, searchResults: [], semanticResults: [], loading: false, error: null }));
  } catch (e) {
    setState("loading", false);
    setState("error", String(e));
  }
}

let searchTimer: ReturnType<typeof setTimeout> | null = null;

export async function searchNotes(query: string): Promise<void> {
  setSearchQuery(query);

  if (searchTimer) clearTimeout(searchTimer);

  if (!query.trim()) {
    setState("searchResults", []);
    return;
  }

  searchTimer = setTimeout(async () => {
    try {
      if (searchMode() === "semantic") {
        const results = await notesSearchSemantic(query);
        setState("semanticResults", reconcile(results));
        setState("searchResults", []);
      } else {
        const results = await notesSearch(query);
        setState("searchResults", reconcile(results));
        setState("semanticResults", []);
      }
    } catch (e) {
      setState("error", String(e));
    }
  }, 200);
}

export async function updateLayout(id: NoteId, layout: NoteLayout): Promise<void> {
  // Optimistic update
  setState("byId", id, "layout", layout);
  try {
    await notesUpdateLayout(id, layout);
  } catch (e) {
    console.error("Failed to persist layout:", e);
  }
}

/**
 * Subscribe to Rust-emitted layout change events (window open/close).
 * Call once at app startup. Returns an unlisten function for cleanup.
 */
export async function initNoteWindowListener(): Promise<() => void> {
  return onNoteLayoutChanged((_id) => {
    // Reload the full note list so layout.visible is up to date.
    void loadNotes();
  });
}

export function upsertNote(note: Note): void {
  setState("byId", note.id, note);
  // Keep visibleIds in sync
  const isVisible = !note.is_trashed && !note.is_archived;
  setState("visibleIds", (ids) => {
    const without = ids.filter((id) => id !== note.id);
    return isVisible ? [note.id, ...without] : without;
  });
}
