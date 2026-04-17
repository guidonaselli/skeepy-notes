import { type Component, createSignal, For, Show } from "solid-js";
import type { Note } from "@/types/note";
import { noteUpdate, noteDelete } from "@/services/tauri.service";
import { ProviderBadge } from "./ProviderBadge";
import { ConflictPanel } from "./ConflictPanel";
import { IconPushPin, IconPencilSimple, IconX } from "./Icons";

interface Props {
  note: Note;
  onClose: () => void;
  onNoteUpdated?: (note: Note) => void;
  onNoteDeleted?: (noteId: string) => void;
}

const COLOR_MAP: Record<string, string> = {
  default:   "#2d2d3d",
  red:       "#4a1a1a",
  orange:    "#3d2a10",
  yellow:    "#3d3510",
  green:     "#1a3d2a",
  teal:      "#1a3a3d",
  blue:      "#1a2a4a",
  dark_blue: "#101a35",
  purple:    "#2a1a4a",
  pink:      "#4a1a3a",
  brown:     "#3a2a1a",
  gray:      "#2a2a2a",
};

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export const NoteDetailPanel: Component<Props> = (props) => {
  const note = () => props.note;
  const bgColor = () => COLOR_MAP[note().color] ?? COLOR_MAP.default;
  // Providers that support inline editing
  const isEditable = () =>
    note().provider_id === "local" ||
    note().provider_id === "onenote" ||
    note().provider_id === "notion";

  const isConflict = () => note().sync_state.status === "conflict";

  const [editMode, setEditMode] = createSignal(false);
  const [editTitle, setEditTitle] = createSignal("");
  const [editBody, setEditBody] = createSignal("");
  const [saving, setSaving] = createSignal(false);
  const [deleting, setDeleting] = createSignal(false);

  function enterEdit() {
    setEditTitle(note().title ?? "");
    setEditBody(note().content.type === "text" ? (note().content as { type: "text"; data: string }).data : "");
    setEditMode(true);
  }

  async function saveEdit() {
    setSaving(true);
    try {
      const updated = await noteUpdate({
        id: note().id,
        title: editTitle() || null,
        content: { type: "text", content: editBody() },
      });
      props.onNoteUpdated?.(updated);
      setEditMode(false);
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!confirm("¿Eliminar esta nota? Se moverá a la papelera.")) return;
    setDeleting(true);
    try {
      await noteDelete(note().id);
      props.onNoteDeleted?.(note().id);
      props.onClose();
    } finally {
      setDeleting(false);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (editMode()) {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        saveEdit();
      }
      if (e.key === "Escape") {
        setEditMode(false);
      }
    }
  }

  return (
    <>
      {/* Backdrop */}
      <div class="detail-backdrop" onClick={() => { if (!editMode()) props.onClose(); }} />

      {/* Panel */}
      <div class="detail-panel" style={{ "background-color": bgColor() }} onKeyDown={handleKeyDown}>
        {/* Header */}
        <div class="detail-panel__header">
          <div class="detail-panel__meta">
            <ProviderBadge providerId={note().provider_id} />
            <Show when={note().is_pinned}>
              <span class="detail-panel__pin" title="Pinned"><IconPushPin size={13} /></span>
            </Show>
          </div>
          <div class="detail-panel__actions">
            <Show when={isEditable() && !editMode() && !isConflict()}>
              <button class="detail-panel__action-btn" title="Editar" onClick={enterEdit}><IconPencilSimple size={15} /></button>
            </Show>
            <button class="detail-panel__action-btn detail-panel__action-btn--danger" title="Eliminar" onClick={handleDelete} disabled={deleting()}>🗑</button>
            <Show when={editMode()}>
              <button class="btn btn--primary btn--small" onClick={saveEdit} disabled={saving()}>
                {saving() ? "…" : "Guardar"}
              </button>
              <button class="btn btn--small" onClick={() => setEditMode(false)}>Cancelar</button>
            </Show>
            <button class="detail-panel__close" onClick={props.onClose} aria-label="Cerrar"><IconX size={15} /></button>
          </div>
        </div>

        {/* Edit mode */}
        <Show when={editMode()}>
          <div class="detail-panel__editor">
            <input
              class="detail-panel__editor-title"
              type="text"
              placeholder="Título (opcional)"
              value={editTitle()}
              onInput={(e) => setEditTitle(e.currentTarget.value)}
            />
            <textarea
              class="detail-panel__editor-body"
              value={editBody()}
              onInput={(e) => setEditBody(e.currentTarget.value)}
            />
          </div>
        </Show>

        {/* Conflict resolution panel — supersedes normal view */}
        <Show when={isConflict() && !editMode()}>
          <ConflictPanel
            note={note()}
            onResolved={(updated) => {
              props.onNoteUpdated?.(updated);
            }}
          />
        </Show>

        {/* Read mode */}
        <Show when={!editMode() && !isConflict()}>
          {/* Title */}
          <Show when={note().title}>
            <h2 class="detail-panel__title">{note().title}</h2>
          </Show>

          {/* Read-only notice for non-editable providers */}
          <Show when={!isEditable()}>
            <p class="detail-panel__readonly-notice">
              Esta nota es de <strong>{note().provider_id}</strong>. La edición inline no está disponible — editala desde la app original.
            </p>
          </Show>

          {/* Content */}
          <div class="detail-panel__body">
            <Show
              when={note().content.type === "checklist"}
              fallback={
                <p class="detail-panel__text">
                  {(note().content as { type: "text"; data: string }).data}
                </p>
              }
            >
              <ul class="detail-panel__checklist">
                <For each={(note().content as { type: "checklist"; data: { text: string; checked: boolean }[] }).data ?? []}>
                  {(item) => (
                    <li class="detail-panel__checklist-item" classList={{ "is-checked": item.checked }}>
                      <span class="check-icon">{item.checked ? "✓" : "○"}</span>
                      <span class="check-text">{item.text}</span>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          </div>

          {/* Labels */}
          <Show when={(note().labels ?? []).length > 0}>
            <div class="detail-panel__labels">
              <For each={note().labels ?? []}>
                {(label) => <span class="label-chip">{label.name}</span>}
              </For>
            </div>
          </Show>

          {/* Timestamps */}
          <div class="detail-panel__timestamps">
            <span>Creada: {formatDate(note().created_at)}</span>
            <span>Modificada: {formatDate(note().updated_at)}</span>
            <Show when={note().synced_at}>
              <span>Sincronizada: {formatDate(note().synced_at!)}</span>
            </Show>
          </div>
        </Show>
      </div>
    </>
  );
};
