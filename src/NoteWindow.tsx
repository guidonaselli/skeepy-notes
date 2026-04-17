import {
  type Component,
  createEffect,
  createResource,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Note, NoteColor } from "@/types/note";
import "./styles/note-window.css";

interface Props {
  noteId: string;
}

// ── Palette ──────────────────────────────────────────────────────────────────

const COLORS: { key: NoteColor; bg: string; header: string; label: string }[] = [
  { key: "default",   bg: "#fff9c4", header: "#fff176", label: "Amarillo"  },
  { key: "green",     bg: "#dcedc8", header: "#c5e1a5", label: "Verde"     },
  { key: "pink",      bg: "#f8bbd0", header: "#f48fb1", label: "Rosa"      },
  { key: "purple",    bg: "#e1bee7", header: "#ce93d8", label: "Violeta"   },
  { key: "blue",      bg: "#bbdefb", header: "#90caf9", label: "Azul"      },
  { key: "teal",      bg: "#b2ebf2", header: "#80deea", label: "Turquesa"  },
  { key: "red",       bg: "#ffcdd2", header: "#ef9a9a", label: "Rojo"      },
  { key: "orange",    bg: "#ffe0b2", header: "#ffcc80", label: "Naranja"   },
  { key: "dark_blue", bg: "#c5cae9", header: "#9fa8da", label: "Índigo"    },
  { key: "brown",     bg: "#d7ccc8", header: "#bcaaa4", label: "Marrón"    },
  { key: "gray",      bg: "#f5f5f5", header: "#e0e0e0", label: "Gris"      },
];

const COLOR_BG: Record<string, string>  = Object.fromEntries(COLORS.map(c => [c.key, c.bg]));
const COLOR_HDR: Record<string, string> = Object.fromEntries(COLORS.map(c => [c.key, c.header]));

// ── Component ─────────────────────────────────────────────────────────────────

const NoteWindow: Component<Props> = (props) => {
  const [note, { mutate, refetch }] = createResource<Note | null, string>(
    () => props.noteId,
    (id: string) => invoke<Note | null>("note_get", { id })
  );

  const [editMode,    setEditMode]    = createSignal(false);
  const [editTitle,   setEditTitle]   = createSignal("");
  const [editBody,    setEditBody]    = createSignal("");
  const [saving,      setSaving]      = createSignal(false);
  const [showPalette, setShowPalette] = createSignal(false);
  const [pinned,      setPinned]      = createSignal(false);

  // ── Drag — use startDragging() (Tauri 2.x recommended for decorations:false) ─
  function onHeaderMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;           // left button only
    const target = e.target as HTMLElement;
    if (target.closest("button")) return;  // don't drag when clicking a button
    e.preventDefault();
    getCurrentWindow().startDragging().catch(() => {});
  }

  // ── Persist position / size on window move or resize ─────────────────────
  let persistTimer: ReturnType<typeof setTimeout> | undefined;
  let unlistenMoved: (() => void) | undefined;
  let unlistenResized: (() => void) | undefined;

  // Initialize pinned from layout once the note resource resolves.
  // createEffect fires reactively when note() transitions undefined → Note.
  let pinnedInitialized = false;
  createEffect(() => {
    const n = note();
    if (n && !pinnedInitialized) {
      pinnedInitialized = true;
      setPinned(n.layout.always_on_top);
    }
  });

  // ── Always-on-top (pin) toggle ────────────────────────────────────────────
  async function togglePin() {
    const n = note();
    if (!n) return;
    const next = !pinned();
    setPinned(next);
    try {
      await getCurrentWindow().setAlwaysOnTop(next);
      await invoke("notes_update_layout", {
        id: n.id,
        layout: { ...n.layout, always_on_top: next, visible: true },
      });
      mutate({ ...n, layout: { ...n.layout, always_on_top: next } });
    } catch (_) {
      setPinned(!next); // revert on error
    }
  }

  onMount(() => {
    const win = getCurrentWindow();

    win.onMoved(async ({ payload }) => {
      clearTimeout(persistTimer);
      persistTimer = setTimeout(async () => {
        const n = note();
        if (!n) return;
        try {
          const sf = await win.scaleFactor();
          await invoke("notes_update_layout", {
            id: n.id,
            layout: { ...n.layout, position: { x: payload.x / sf, y: payload.y / sf }, visible: true },
          });
        } catch (_) {}
      }, 600);
    }).then(fn => { unlistenMoved = fn; });

    win.onResized(async ({ payload }) => {
      clearTimeout(persistTimer);
      persistTimer = setTimeout(async () => {
        const n = note();
        if (!n) return;
        try {
          const sf = await win.scaleFactor();
          await invoke("notes_update_layout", {
            id: n.id,
            layout: { ...n.layout, size: { width: payload.width / sf, height: payload.height / sf }, visible: true },
          });
        } catch (_) {}
      }, 600);
    }).then(fn => { unlistenResized = fn; });
  });

  onCleanup(() => {
    clearTimeout(persistTimer);
    unlistenMoved?.();
    unlistenResized?.();
  });

  // ── Close ─────────────────────────────────────────────────────────────────
  async function closeNote() {
    const n = note();
    if (n) {
      try {
        await invoke("notes_update_layout", { id: n.id, layout: { ...n.layout, visible: false } });
      } catch (_) {}
    }
    await getCurrentWindow().close();
  }

  // ── Color ─────────────────────────────────────────────────────────────────
  async function pickColor(color: NoteColor) {
    setShowPalette(false);
    const n = note();
    if (!n) return;
    try {
      const updated = await invoke<Note>("note_update_color", { id: n.id, color });
      mutate(updated);
    } catch (_) {}
  }

  // ── Edit ──────────────────────────────────────────────────────────────────
  function enterEdit() {
    const n = note();
    if (!n) return;
    setEditTitle(n.title ?? "");
    const c = n.content;
    setEditBody(c.type === "text" ? c.data : "");
    setEditMode(true);
  }

  async function saveEdit() {
    const n = note();
    if (!n) return;
    setSaving(true);
    try {
      await invoke("note_update", {
        id: n.id,
        title: editTitle() || null,
        content: { type: "text", content: editBody() },
        color: null,
      });
      setEditMode(false);
      refetch();
    } finally {
      setSaving(false);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") { e.preventDefault(); saveEdit(); }
    if (e.key === "Escape") { setEditMode(false); setShowPalette(false); }
  }

  // ── Render ────────────────────────────────────────────────────────────────
  return (
    <Show when={note()} fallback={<div class="note-win__loading">…</div>}>
      {(n) => {
        const bg  = () => COLOR_BG[n().color]  ?? COLOR_BG.default;
        const hdr = () => COLOR_HDR[n().color] ?? COLOR_HDR.default;
        const editable = () => ["local", "onenote", "notion"].includes(n().provider_id);

        return (
          <div
            class="note-win"
            style={{ "background-color": bg() }}
            onKeyDown={handleKeyDown}
            onClick={() => setShowPalette(false)}
          >
            {/* ── Header ── */}
            <div
              class="note-win__header"
              style={{ "background-color": hdr() }}
              onMouseDown={onHeaderMouseDown}
            >
              <Show when={n().title && !editMode()}>
                <span class="note-win__title">{n().title}</span>
              </Show>

              <div class="note-win__header-actions">
                {/* Pin / always-on-top toggle */}
                <Show when={!editMode()}>
                  <button
                    class="note-win__btn note-win__btn--pin"
                    classList={{ "is-active": pinned() }}
                    title={pinned() ? "Desanclar ventana" : "Anclar encima de todo"}
                    style={{ "background-color": hdr() }}
                    onClick={(e) => { e.stopPropagation(); togglePin(); }}
                  >
                    📌
                  </button>
                </Show>

                {/* Color picker toggle */}
                <Show when={!editMode()}>
                  <button
                    class="note-win__btn note-win__btn--color"
                    title="Cambiar color"
                    style={{ "background-color": hdr() }}
                    onClick={(e) => { e.stopPropagation(); setShowPalette(v => !v); }}
                  >
                    <span class="note-win__color-dot" style={{ "background-color": bg() }} />
                  </button>
                </Show>

                {/* Edit / Save / Cancel */}
                <Show when={editable() && !editMode()}>
                  <button class="note-win__btn" title="Editar (Ctrl+E)" onClick={enterEdit}>✎</button>
                </Show>
                <Show when={editMode()}>
                  <button
                    class="note-win__btn note-win__btn--save"
                    title="Guardar (Ctrl+S)"
                    onClick={saveEdit}
                    disabled={saving()}
                  >
                    {saving() ? "…" : "✓"}
                  </button>
                  <button class="note-win__btn" title="Cancelar (Esc)" onClick={() => setEditMode(false)}>✕</button>
                </Show>

                {/* Close */}
                <Show when={!editMode()}>
                  <button class="note-win__btn note-win__btn--close" title="Cerrar" onClick={closeNote}>✕</button>
                </Show>
              </div>
            </div>

            {/* ── Color palette popup ── */}
            <Show when={showPalette()}>
              <div class="note-win__palette" onClick={(e) => e.stopPropagation()}>
                <For each={COLORS}>
                  {(c) => (
                    <button
                      class="note-win__palette-swatch"
                      classList={{ "is-active": n().color === c.key }}
                      style={{ "background-color": c.bg, outline: `2px solid ${c.header}` }}
                      title={c.label}
                      onClick={() => pickColor(c.key)}
                    />
                  )}
                </For>
              </div>
            </Show>

            {/* ── Body ── */}
            <Show
              when={editMode()}
              fallback={
                <div class="note-win__body">
                  <Show
                    when={n().content.type === "checklist"}
                    fallback={
                      <p class="note-win__text">
                        {(n().content as { type: "text"; data: string }).data}
                      </p>
                    }
                  >
                    <ul class="note-win__checklist">
                      <For each={(n().content as { type: "checklist"; data: { text: string; checked: boolean }[] }).data ?? []}>
                        {(item) => (
                          <li class="note-win__check-item" classList={{ "is-checked": item.checked }}>
                            <span class="note-win__check-icon">{item.checked ? "✓" : "○"}</span>
                            {item.text}
                          </li>
                        )}
                      </For>
                    </ul>
                  </Show>
                </div>
              }
            >
              <div class="note-win__editor">
                <input
                  class="note-win__editor-title"
                  type="text"
                  placeholder="Título (opcional)"
                  value={editTitle()}
                  onInput={(e) => setEditTitle(e.currentTarget.value)}
                />
                <textarea
                  class="note-win__editor-body"
                  value={editBody()}
                  onInput={(e) => setEditBody(e.currentTarget.value)}
                  autofocus
                />
              </div>
            </Show>
          </div>
        );
      }}
    </Show>
  );
};

export default NoteWindow;
