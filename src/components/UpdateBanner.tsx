import { type Component, createSignal, onCleanup, onMount, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const UpdateBanner: Component = () => {
  const [update, setUpdate] = createSignal<{ version: string; notes: string } | null>(null);
  const [installing, setInstalling] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    const unlisten = await listen<{ version: string; notes: string }>("update://available", (e) => {
      setUpdate(e.payload);
    });
    onCleanup(unlisten);
  });

  async function install() {
    setInstalling(true);
    setError(null);
    try {
      await invoke("updater_install");
      // App restarts automatically after install on Windows
    } catch (e) {
      setError(String(e));
      setInstalling(false);
    }
  }

  return (
    <Show when={update()}>
      {(u) => (
        <div class="update-banner">
          <span class="update-banner__text">
            Nueva versión <strong>{u().version}</strong> disponible
            <Show when={error()}>
              <span class="update-banner__error"> — {error()}</span>
            </Show>
          </span>
          <div class="update-banner__actions">
            <button
              class="btn btn--small btn--primary"
              onClick={install}
              disabled={installing()}
            >
              {installing() ? "Instalando…" : "Instalar y reiniciar"}
            </button>
            <button
              class="btn btn--icon btn--small"
              title="Recordarme después"
              onClick={() => setUpdate(null)}
            >
              ✕
            </button>
          </div>
        </div>
      )}
    </Show>
  );
};

export default UpdateBanner;
