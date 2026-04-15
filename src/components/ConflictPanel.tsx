import { type Component, createResource, Show } from "solid-js";
import type { Note } from "@/types/note";
import { noteGetConflict, noteResolveConflict } from "@/services/tauri.service";

interface Props {
  note: Note;
  onResolved: (updated: Note) => void;
}

function fmt(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export const ConflictPanel: Component<Props> = (props) => {
  const [info] = createResource(() => props.note.id, noteGetConflict);

  async function resolve(keep: "local" | "remote") {
    try {
      const updated = await noteResolveConflict(props.note.id, keep);
      props.onResolved(updated);
    } catch (e) {
      console.error("resolve conflict failed", e);
    }
  }

  return (
    <div class="conflict-panel">
      <div class="conflict-panel__banner">
        ⚠ Conflicto de sincronización — este nota fue editada en Skeepy y en el
        proveedor al mismo tiempo. Elegí qué versión conservar.
      </div>

      <Show when={info()} fallback={<p class="conflict-panel__loading">Cargando…</p>}>
        {(data) => (
          <div class="conflict-panel__columns">
            {/* Local side */}
            <div class="conflict-panel__side conflict-panel__side--local">
              <div class="conflict-panel__side-header">
                <span class="conflict-panel__side-label">Tu versión (Skeepy)</span>
                <span class="conflict-panel__side-date">{fmt(data().local_updated_at)}</span>
              </div>
              <Show when={data().local_title}>
                <p class="conflict-panel__title">{data().local_title}</p>
              </Show>
              <pre class="conflict-panel__content">{data().local_content_text}</pre>
              <button
                class="btn btn--primary conflict-panel__keep-btn"
                onClick={() => resolve("local")}
              >
                Conservar esta versión
              </button>
            </div>

            {/* Divider */}
            <div class="conflict-panel__divider" aria-hidden="true">VS</div>

            {/* Remote side */}
            <div class="conflict-panel__side conflict-panel__side--remote">
              <div class="conflict-panel__side-header">
                <span class="conflict-panel__side-label">Versión remota ({props.note.provider_id})</span>
                <span class="conflict-panel__side-date">{fmt(data().remote_updated_at)}</span>
              </div>
              <Show when={data().remote_title}>
                <p class="conflict-panel__title">{data().remote_title}</p>
              </Show>
              <pre class="conflict-panel__content">{data().remote_content_text}</pre>
              <button
                class="btn btn--primary conflict-panel__keep-btn"
                onClick={() => resolve("remote")}
              >
                Conservar esta versión
              </button>
            </div>
          </div>
        )}
      </Show>
    </div>
  );
};
