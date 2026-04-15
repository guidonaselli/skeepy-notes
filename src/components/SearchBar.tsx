import { type Component } from "solid-js";
import { searchNotes, notesStore, setSearchMode } from "@/stores/notes.store";

export const SearchBar: Component = () => {
  const toggleMode = () => {
    const next = notesStore.mode === "fts" ? "semantic" : "fts";
    setSearchMode(next);
    // Re-run the current query in the new mode
    if (notesStore.query) {
      searchNotes(notesStore.query);
    }
  };

  return (
    <div class="search-bar">
      <span class="search-bar__icon">🔍</span>
      <input
        type="search"
        class="search-bar__input"
        placeholder={notesStore.mode === "semantic" ? "Búsqueda semántica…" : "Buscar notas…"}
        value={notesStore.query}
        onInput={(e) => searchNotes(e.currentTarget.value)}
        aria-label="Buscar notas"
      />
      <button
        class="search-bar__mode-btn"
        classList={{ "is-semantic": notesStore.mode === "semantic" }}
        title={notesStore.mode === "semantic"
          ? "Modo semántico (busca por significado) — click para cambiar a exacto"
          : "Modo exacto (FTS5) — click para cambiar a semántico"}
        onClick={toggleMode}
      >
        {notesStore.mode === "semantic" ? "✦" : "≈"}
      </button>
    </div>
  );
};
