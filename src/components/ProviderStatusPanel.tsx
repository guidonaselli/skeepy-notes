import { type Component, createResource, createSignal, For, onCleanup, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { providersStatus, syncProvider } from "@/services/tauri.service";
import type { ProviderStatusInfo, ProviderStatusTag, SyncProgressEvent } from "@/types/note";

// ─── Helpers ──────────────────────────────────────────────────────────────────

function statusLabel(s: ProviderStatusTag): string {
  switch (s.status) {
    case "active":          return "Activo";
    case "unauthenticated": return "Sin autenticar";
    case "rate_limited":    return "Rate limited";
    case "error":           return `Error: ${s.message}`;
    case "disabled":        return "Desactivado";
  }
}

function statusColor(s: ProviderStatusTag): string {
  switch (s.status) {
    case "active":     return "#4caf50";
    case "error":      return "#f44336";
    case "disabled":   return "#9e9e9e";
    default:           return "#ff9800";
  }
}

function stabilityBadge(stability: ProviderStatusInfo["capabilities"]["stability"]): string {
  return stability === "experimental" ? " (exp.)" : stability === "deprecated" ? " (dep.)" : "";
}

function formatTimestamp(ts: string | null): string {
  if (!ts) return "nunca";
  const d = new Date(ts);
  const diffMs = Date.now() - d.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1)  return "hace un momento";
  if (diffMin < 60) return `hace ${diffMin} min`;
  const diffH = Math.floor(diffMin / 60);
  if (diffH < 24)   return `hace ${diffH}h`;
  return `hace ${Math.floor(diffH / 24)}d`;
}

// ─── Component ────────────────────────────────────────────────────────────────

export const ProviderStatusPanel: Component = () => {
  const [providers, { refetch }] = createResource<ProviderStatusInfo[]>(providersStatus);
  const [syncing, setSyncing] = createSignal<string | null>(null);

  // Update panel when a sync event arrives
  const unlistenPromise = listen<SyncProgressEvent>("sync://progress", () => {
    refetch();
  });

  onCleanup(async () => {
    const unlisten = await unlistenPromise;
    unlisten();
  });

  async function handleSync(providerId: string) {
    setSyncing(providerId);
    try {
      await syncProvider(providerId);
      await refetch();
    } finally {
      setSyncing(null);
    }
  }

  return (
    <div class="provider-panel">
      <For each={providers()} fallback={
        <p class="settings-panel__note">No hay providers activos.</p>
      }>
        {(p) => (
          <div class="provider-panel__row">
            <div class="provider-panel__info">
              <span class="provider-panel__name">{p.display_name}</span>
              <Show when={p.capabilities.stability !== "stable"}>
                <span class="provider-panel__stability">
                  {p.capabilities.stability === "experimental" ? "experimental" : "deprecated"}
                </span>
              </Show>
              <span
                class="provider-panel__status"
                style={{ color: statusColor(p.status) }}
              >
                {statusLabel(p.status)}
              </span>
              <Show when={p.last_error}>
                <span class="provider-panel__error">{p.last_error}</span>
              </Show>
              <span class="provider-panel__last-sync">
                Última sync: {formatTimestamp(p.last_sync_at)}
              </span>
            </div>
            <button
              class="btn btn--small"
              disabled={syncing() === p.id}
              onClick={() => handleSync(p.id)}
            >
              {syncing() === p.id ? "…" : "Sync"}
            </button>
          </div>
        )}
      </For>
    </div>
  );
};
