import { type Component, For, Show } from "solid-js";
import type { Note } from "@/types/note";
import { notesStore } from "@/stores/notes.store";
import { NoteCard } from "./NoteCard";
import { WelcomeScreen } from "./WelcomeScreen";

interface Props {
  onNoteExpand?: (note: Note) => void;
  /** When set, only notes that carry this label name are shown. */
  labelFilter?: string | null;
  onOpenSettings?: () => void;
  onCreateNote?: () => void;
}

export const NoteGrid: Component<Props> = (props) => {
  const baseNotes = () => {
    if (!notesStore.isSearching) return notesStore.visibleNotes;
    if (notesStore.mode === "semantic") return notesStore.semanticResults.map((r) => r.note);
    return notesStore.searchResults.map((r) => r.note);
  };

  const notes = () => {
    const label = props.labelFilter;
    if (!label) return baseNotes();
    return baseNotes().filter((n) => (n.labels ?? []).some((l) => l.name === label));
  };

  const showWelcome = () =>
    !notesStore.loading && !notesStore.isSearching && notes().length === 0;

  return (
    <div class="note-grid">
      <Show when={notesStore.loading}>
        <div class="note-grid__loading">Cargando notas…</div>
      </Show>
      <Show when={notesStore.error}>
        <div class="note-grid__error">Error: {notesStore.error}</div>
      </Show>
      <Show when={!notesStore.loading && notesStore.isSearching && notes().length === 0}>
        <div class="note-grid__empty">
          {notesStore.mode === "semantic"
            ? "Sin resultados semánticos — el índice puede estar construyéndose."
            : "Sin resultados para esta búsqueda."}
        </div>
      </Show>
      <Show when={showWelcome()}>
        <WelcomeScreen
          onOpenSettings={props.onOpenSettings ?? (() => {})}
          onCreateNote={props.onCreateNote ?? (() => {})}
        />
      </Show>
      <For each={notes()}>
        {(note) => (
          <NoteCard
            note={note}
            onExpand={props.onNoteExpand ? () => props.onNoteExpand!(note) : undefined}
          />
        )}
      </For>
    </div>
  );
};
