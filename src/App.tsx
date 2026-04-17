import { type Component, createResource, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { labelsGetAll, onNoteCreateRequested } from "@/services/tauri.service";
import { listen } from "@tauri-apps/api/event";
import type { Note, SyncProgressEvent } from "@/types/note";
import { initNoteWindowListener, loadNotes } from "@/stores/notes.store";
import { initSyncListener, triggerSync } from "@/stores/sync.store";
import { CreateNoteModal } from "@/components/CreateNoteModal";
import { NoteDetailPanel } from "@/components/NoteDetailPanel";
import { NoteGrid } from "@/components/NoteGrid";
import { SearchBar } from "@/components/SearchBar";
import { Settings } from "@/components/Settings";
import UpdateBanner from "@/components/UpdateBanner";
import { IconPlus, IconArrowClockwise, IconGearSix, IconX } from "@/components/Icons";
import "./styles/global.css";

const App: Component = () => {
  const [showSettings, setShowSettings] = createSignal(false);
  const [selectedNote, setSelectedNote] = createSignal<Note | null>(null);
  const [showCreateModal, setShowCreateModal] = createSignal(false);
  const [syncErrors, setSyncErrors] = createSignal<{ provider: string; message: string }[]>([]);
  const [labelFilter, setLabelFilter] = createSignal<string | null>(null);
  const [allLabels] = createResource(labelsGetAll);

  let unlistenNoteWindow: (() => void) | undefined;

  onMount(async () => {
    await initSyncListener();
    unlistenNoteWindow = await initNoteWindowListener();
    await loadNotes();
    await triggerSync();

    // Global Ctrl+N shortcut to create a new note
    window.addEventListener("keydown", handleGlobalKeyDown);
  });

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "n" && !showCreateModal()) {
      e.preventDefault();
      setShowCreateModal(true);
    }
  }

  // Open create modal when tray "Nueva nota" is clicked.
  const unlistenCreatePromise = onNoteCreateRequested(() => {
    if (!showCreateModal()) setShowCreateModal(true);
  });

  // Listen for sync errors and surface them as dismissable banners.
  const unlistenPromise = listen<SyncProgressEvent>("sync://progress", (e) => {
    if (e.payload.status === "error" && e.payload.error) {
      setSyncErrors((prev) => {
        const filtered = prev.filter((x) => x.provider !== e.payload.provider_id);
        return [...filtered, { provider: e.payload.provider_id, message: e.payload.error! }];
      });
    } else if (e.payload.status === "ok") {
      setSyncErrors((prev) => prev.filter((x) => x.provider !== e.payload.provider_id));
    }
  });

  onCleanup(async () => {
    (await unlistenPromise)();
    (await unlistenCreatePromise)();
    unlistenNoteWindow?.();
    window.removeEventListener("keydown", handleGlobalKeyDown);
  });

  return (
    <div class="app">
      <header class="app__toolbar">
        <SearchBar />
        <div class="app__toolbar-actions">
          <button
            class="btn btn--icon"
            title="Nueva nota (Ctrl+N)"
            onClick={() => setShowCreateModal(true)}
          >
            <IconPlus size={16} />
          </button>
          <button
            class="btn btn--icon"
            title="Sincronizar"
            onClick={triggerSync}
          >
            <IconArrowClockwise size={16} />
          </button>
          <button
            class="btn btn--icon"
            title="Configuración"
            onClick={() => setShowSettings((v) => !v)}
          >
            <IconGearSix size={16} />
          </button>
        </div>
      </header>

      <UpdateBanner />

      <Show when={syncErrors().length > 0}>
        <div class="app__sync-errors">
          {syncErrors().map((err) => (
            <div class="sync-error-banner">
              <span>
                <strong>{err.provider}</strong>: {err.message}
              </span>
              <div class="sync-error-banner__actions">
                <button class="btn btn--icon btn--small" title="Reintentar" onClick={triggerSync}><IconArrowClockwise size={14} /></button>
                <button
                  class="btn btn--icon btn--small"
                  title="Cerrar"
                  onClick={() => setSyncErrors((p) => p.filter((x) => x.provider !== err.provider))}
                ><IconX size={14} /></button>
              </div>
            </div>
          ))}
        </div>
      </Show>

      <Show when={(allLabels() ?? []).length > 0}>
        <div class="app__label-bar">
          <button
            class={`app__label-chip${!labelFilter() ? " app__label-chip--active" : ""}`}
            onClick={() => setLabelFilter(null)}
          >
            Todas
          </button>
          <For each={allLabels() ?? []}>
            {(label) => (
              <button
                class={`app__label-chip${labelFilter() === label.name ? " app__label-chip--active" : ""}`}
                onClick={() => setLabelFilter((f) => f === label.name ? null : label.name)}
              >
                {label.name}
                <span class="app__label-chip__count">{label.note_count}</span>
              </button>
            )}
          </For>
        </div>
      </Show>

      <main class="app__canvas">
        <NoteGrid
          onNoteExpand={(note) => setSelectedNote(note)}
          labelFilter={labelFilter()}
          onOpenSettings={() => setShowSettings(true)}
          onCreateNote={() => setShowCreateModal(true)}
        />
      </main>

      <Show when={showCreateModal()}>
        <CreateNoteModal
          onCreated={async (note) => {
            setShowCreateModal(false);
            await loadNotes();
            // Open the new note as a sticky note window on the desktop
            try {
              await invoke("note_window_show", { id: note.id });
            } catch (_) {
              // Fallback: show detail panel if window creation fails
              setSelectedNote(note);
            }
          }}
          onClose={() => setShowCreateModal(false)}
        />
      </Show>

      <Show when={selectedNote()}>
        <NoteDetailPanel
          note={selectedNote()!}
          onClose={() => setSelectedNote(null)}
          onNoteUpdated={async (updated) => {
            setSelectedNote(updated);
            await loadNotes();
          }}
          onNoteDeleted={async () => {
            setSelectedNote(null);
            await loadNotes();
          }}
        />
      </Show>

      <Show when={showSettings()}>
        <div class="app__settings-overlay" onClick={() => setShowSettings(false)}>
          <div class="app__settings-panel" onClick={(e) => e.stopPropagation()}>
            <Settings />
          </div>
        </div>
      </Show>
    </div>
  );
};

export default App;
