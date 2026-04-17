import { type Component, createSignal, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { Note, Point } from "@/types/note";
import { updateLayout } from "@/stores/notes.store";
import { ProviderBadge } from "./ProviderBadge";
import { IconPushPin, IconArrowSquareOut, IconArrowsOutSimple } from "./Icons";

interface Props {
  note: Note;
  onExpand?: () => void;
}

// Note body background — Windows Sticky Notes light palette
const COLOR_MAP: Record<string, string> = {
  default:   "#fff9c4",
  red:       "#ffcdd2",
  orange:    "#ffe0b2",
  yellow:    "#fff9c4",
  green:     "#dcedc8",
  teal:      "#b2ebf2",
  blue:      "#bbdefb",
  dark_blue: "#c5cae9",
  purple:    "#e1bee7",
  pink:      "#f8bbd0",
  brown:     "#d7ccc8",
  gray:      "#f5f5f5",
};

// Header strip — slightly more saturated version of the note color
const HEADER_COLOR_MAP: Record<string, string> = {
  default:   "#fff176",
  red:       "#ef9a9a",
  orange:    "#ffcc80",
  yellow:    "#fff176",
  green:     "#c5e1a5",
  teal:      "#80deea",
  blue:      "#90caf9",
  dark_blue: "#9fa8da",
  purple:    "#ce93d8",
  pink:      "#f48fb1",
  brown:     "#bcaaa4",
  gray:      "#e0e0e0",
};

export const NoteCard: Component<Props> = (props) => {
  const note = () => props.note;
  const layout = () => note().layout;

  const bgColor = () => COLOR_MAP[note().color] ?? COLOR_MAP.default;
  const headerColor = () => HEADER_COLOR_MAP[note().color] ?? HEADER_COLOR_MAP.default;
  const position = () => layout().position ?? { x: 20, y: 20 };
  const width = () => layout().size?.width ?? 280;
  const height = () => layout().size?.height ?? 220;

  // ── Content preview ────────────────────────────────────────────────────────
  const contentPreview = () => {
    const c = note().content;
    if (c.type === "text") return c.data.slice(0, 200);
    return (c.data ?? []).map((i) => (i.checked ? "✓ " : "○ ") + i.text).join("\n");
  };

  const isChecklist = () => note().content.type === "checklist";

  // Sync state badge
  const syncBadge = (): { icon: string; title: string; cls: string } | null => {
    const s = note().sync_state;
    if (s.status === "conflict")    return { icon: "⚡", title: "Conflicto — abrí la nota para resolver", cls: "conflict" };
    if (s.status === "local_ahead") return { icon: "↑", title: "Cambios pendientes de sync", cls: "ahead" };
    if (s.status === "sync_error")  return { icon: "!", title: `Error de sync: ${s.message}`, cls: "error" };
    return null;
  };

  // ── Drag ──────────────────────────────────────────────────────────────────
  let dragStart: { mouseX: number; mouseY: number; noteX: number; noteY: number } | null = null;
  const [isDragging, setIsDragging] = createSignal(false);

  function onMouseDown(e: MouseEvent) {
    // Only drag from the header, not from content/buttons
    dragStart = {
      mouseX: e.clientX,
      mouseY: e.clientY,
      noteX: position().x,
      noteY: position().y,
    };
    setIsDragging(true);
    e.preventDefault();
  }

  function onMouseMove(e: MouseEvent) {
    if (!dragStart) return;
    const dx = e.clientX - dragStart.mouseX;
    const dy = e.clientY - dragStart.mouseY;
    const newPos: Point = {
      x: Math.max(0, dragStart.noteX + dx),
      y: Math.max(0, dragStart.noteY + dy),
    };
    // Optimistic update via store
    updateLayout(note().id, {
      ...layout(),
      position: newPos,
    });
  }

  function onMouseUp() {
    if (!dragStart) return;
    dragStart = null;
    setIsDragging(false);
    // Layout was already persisted optimistically in onMouseMove's last call
  }

  // Register global listeners when dragging starts
  function startDrag(e: MouseEvent) {
    onMouseDown(e);
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", stopDrag, { once: true });
  }

  function stopDrag() {
    onMouseUp();
    window.removeEventListener("mousemove", onMouseMove);
  }

  return (
    <div
      class="note-card"
      classList={{ dragging: isDragging() }}
      style={{
        position: "absolute",
        left: `${position().x}px`,
        top: `${position().y}px`,
        width: `${width()}px`,
        "min-height": `${height()}px`,
        "background-color": bgColor(),
      }}
    >
      {/* Drag handle — header strip (Windows Sticky Notes style) */}
      <div
        class="note-card__header"
        onMouseDown={startDrag}
        style={{ "background-color": headerColor() }}
      >
        <Show when={note().is_pinned}>
          <span class="note-card__pin" title="Pinned">
            <IconPushPin size={12} />
          </span>
        </Show>
        <Show when={note().title}>
          <h3 class="note-card__title">{note().title}</h3>
        </Show>
        <ProviderBadge providerId={note().provider_id} />
        <Show when={syncBadge()}>
          {(badge) => (
            <span
              class={`note-card__sync-badge note-card__sync-badge--${badge().cls}`}
              title={badge().title}
            >
              {badge().icon}
            </span>
          )}
        </Show>
        <button
          class="note-card__expand"
          classList={{ "note-card__expand--active": layout().visible }}
          title={layout().visible ? "Enfocar ventana" : "Abrir en escritorio"}
          onClick={(e) => {
            e.stopPropagation();
            invoke("note_window_show", { id: note().id }).catch(() => {
              props.onExpand?.();
            });
          }}
        >
          <IconArrowSquareOut size={13} />
        </button>
        <Show when={props.onExpand}>
          <button
            class="note-card__expand"
            title="Ver detalle"
            onClick={(e) => { e.stopPropagation(); props.onExpand?.(); }}
          >
            <IconArrowsOutSimple size={13} />
          </button>
        </Show>
      </div>

      {/* Content */}
      <div class="note-card__body">
        <Show when={!isChecklist()} fallback={
          <ul class="note-card__checklist">
            <For each={(note().content as { type: "checklist"; data: { text: string; checked: boolean }[] }).data ?? []}>
              {(item) => (
                <li classList={{ checked: item.checked }}>
                  <span class="check-icon">{item.checked ? "✓" : "○"}</span>
                  {item.text}
                </li>
              )}
            </For>
          </ul>
        }>
          <p class="note-card__text">{contentPreview()}</p>
        </Show>
      </div>

      {/* Labels */}
      <Show when={(note().labels ?? []).length > 0}>
        <div class="note-card__labels">
          <For each={note().labels ?? []}>
            {(label) => <span class="label-chip">{label.name}</span>}
          </For>
        </div>
      </Show>
    </div>
  );
};
