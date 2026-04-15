import { type Component, createSignal, onMount } from "solid-js";
import type { Note } from "@/types/note";
import { noteCreate } from "@/services/tauri.service";

interface Props {
  onCreated: (note: Note) => void;
  onClose: () => void;
}

export const CreateNoteModal: Component<Props> = (props) => {
  const [title, setTitle] = createSignal("");
  const [body, setBody] = createSignal("");
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal("");

  let titleRef: HTMLInputElement | undefined;

  onMount(() => {
    titleRef?.focus();
  });

  async function handleCreate() {
    if (!body().trim() && !title().trim()) {
      setError("Escribí algo antes de guardar.");
      return;
    }

    setSaving(true);
    setError("");

    try {
      const note = await noteCreate({
        title: title() || null,
        content: { type: "text", content: body() },
      });
      props.onCreated(note);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      props.onClose();
    }
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      handleCreate();
    }
  }

  return (
    <>
      <div class="modal-backdrop" onClick={props.onClose} />
      <div class="modal-panel" onKeyDown={handleKeyDown}>
        <div class="modal-panel__header">
          <h2 class="modal-panel__title">Nueva nota</h2>
          <button class="modal-panel__close" onClick={props.onClose} aria-label="Cerrar">✕</button>
        </div>

        <div class="modal-panel__body">
          <input
            ref={titleRef}
            class="modal-panel__input"
            type="text"
            placeholder="Título (opcional)"
            value={title()}
            onInput={(e) => setTitle(e.currentTarget.value)}
          />
          <textarea
            class="modal-panel__textarea"
            placeholder="Escribí tu nota aquí…"
            rows={8}
            value={body()}
            onInput={(e) => setBody(e.currentTarget.value)}
          />
          {error() && <p class="modal-panel__error">{error()}</p>}
        </div>

        <div class="modal-panel__footer">
          <button class="btn" onClick={props.onClose}>Cancelar</button>
          <button class="btn btn--primary" onClick={handleCreate} disabled={saving()}>
            {saving() ? "Guardando…" : "Crear nota"}
          </button>
        </div>
      </div>
    </>
  );
};
