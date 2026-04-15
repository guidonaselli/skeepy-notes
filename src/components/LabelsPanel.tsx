import { type Component, createResource, createSignal, For, Show } from "solid-js";
import { labelsGetAll, labelRename, labelDelete, type LabelInfo } from "@/services/tauri.service";

export const LabelsPanel: Component = () => {
  const [labels, { refetch }] = createResource<LabelInfo[]>(labelsGetAll);
  const [editing, setEditing] = createSignal<string | null>(null);
  const [editValue, setEditValue] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");

  function startEdit(label: LabelInfo) {
    setEditing(label.name);
    setEditValue(label.name);
    setMessage("");
  }

  async function commitRename() {
    const oldName = editing();
    const newName = editValue().trim();
    if (!oldName || !newName) return;
    if (oldName === newName) { setEditing(null); return; }

    setBusy(true);
    try {
      const count = await labelRename(oldName, newName);
      setMessage(`Renombrada en ${count} nota${count !== 1 ? "s" : ""}.`);
      setEditing(null);
      await refetch();
    } catch (e) {
      setMessage(`Error: ${e}`);
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete(name: string) {
    if (!confirm(`¿Eliminar la label "${name}" de todas las notas locales?`)) return;
    setBusy(true);
    try {
      const count = await labelDelete(name);
      setMessage(`Eliminada de ${count} nota${count !== 1 ? "s" : ""}.`);
      await refetch();
    } catch (e) {
      setMessage(`Error: ${e}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="labels-panel">
      <Show when={labels.loading}>
        <p class="settings-panel__note">Cargando labels…</p>
      </Show>

      <Show when={labels()?.length === 0}>
        <p class="settings-panel__note">No hay labels todavía.</p>
      </Show>

      <For each={labels()}>
        {(label) => (
          <div class="labels-panel__row">
            <Show
              when={editing() === label.name}
              fallback={
                <>
                  <span class="labels-panel__name">{label.name}</span>
                  <span class="labels-panel__meta">
                    {label.note_count} nota{label.note_count !== 1 ? "s" : ""} · {label.providers.join(", ")}
                  </span>
                  <div class="labels-panel__actions">
                    <Show when={label.is_local}>
                      <button class="btn btn--small" onClick={() => startEdit(label)} disabled={busy()}>
                        ✎
                      </button>
                      <button class="btn btn--small btn--danger" onClick={() => handleDelete(label.name)} disabled={busy()}>
                        🗑
                      </button>
                    </Show>
                  </div>
                </>
              }
            >
              <input
                class="settings-panel__text-input"
                style={{ flex: "1" }}
                value={editValue()}
                onInput={(e) => setEditValue(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") commitRename();
                  if (e.key === "Escape") setEditing(null);
                }}
              />
              <button class="btn btn--small btn--primary" onClick={commitRename} disabled={busy()}>
                {busy() ? "…" : "OK"}
              </button>
              <button class="btn btn--small" onClick={() => setEditing(null)}>
                Cancelar
              </button>
            </Show>
          </div>
        )}
      </For>

      <Show when={message()}>
        <p class="settings-panel__note">{message()}</p>
      </Show>
    </div>
  );
};
