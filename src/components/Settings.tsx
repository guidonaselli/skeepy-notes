import { type Component, createEffect, createResource, createSignal, onCleanup, Show } from "solid-js";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { ProviderStatusPanel } from "@/components/ProviderStatusPanel";
import { LabelsPanel } from "@/components/LabelsPanel";
import { open as shellOpen } from "@tauri-apps/plugin-shell";
import { start as oauthStart, cancel as oauthCancel } from "@fabianlars/tauri-plugin-oauth";
import { triggerSync } from "@/stores/sync.store";
import {
  settingsGet,
  settingsSet,
  keepStatus,
  keepRevoke,
  keepStartAuth,
  keepCompleteAuth,
  keepCredentialsGet,
  keepCredentialsSet,
  markdownGetFolder,
  markdownSetFolder,
  obsidianGetVault,
  obsidianSetVault,
  oneNoteStatus,
  oneNoteRevoke,
  oneNoteStartAuth,
  oneNoteCompleteAuth,
  oneNoteCredentialsGet,
  oneNoteCredentialsSet,
  notionStatus,
  notionRevoke,
  notionStartAuth,
  notionCompleteAuth,
  notionCredentialsGet,
  notionCredentialsSet,
  notesExport,
  updaterCheck,
  updaterInstall,
  getDataDir,
  type UpdateInfo,
} from "@/services/tauri.service";
import type { AppSettings } from "@/types/note";

// How long to wait for the user to complete the OAuth flow in the browser (ms).
const OAUTH_TIMEOUT_MS = 5 * 60 * 1000;

type ConnectStatus = "idle" | "connecting" | "error";

export const Settings: Component = () => {
  const [settings, { mutate }] = createResource<AppSettings>(settingsGet);
  const [keepConnected, { refetch: refetchKeepStatus }] = createResource<boolean>(keepStatus);
  const [dataDir] = createResource<string>(getDataDir);
  const [appVersion] = createResource<string>(getVersion);

  // ─── About / Update check ─────────────────────────────────────────────────
  const [checkState, setCheckState] = createSignal<"idle" | "checking" | "uptodate" | "available" | "error">("idle");
  const [updateInfo, setUpdateInfo] = createSignal<UpdateInfo | null>(null);
  const [updateError, setUpdateError] = createSignal("");
  const [installing, setInstalling] = createSignal(false);

  async function handleCheckForUpdates() {
    setCheckState("checking");
    setUpdateInfo(null);
    setUpdateError("");
    try {
      const info = await updaterCheck();
      setUpdateInfo(info);
      setCheckState(info ? "available" : "uptodate");
    } catch (e) {
      setUpdateError(e instanceof Error ? e.message : String(e));
      setCheckState("error");
    }
  }

  async function handleInstallUpdate() {
    setInstalling(true);
    try {
      await updaterInstall();
    } catch (e) {
      setUpdateError(e instanceof Error ? e.message : String(e));
      setCheckState("error");
      setInstalling(false);
    }
  }
  const [saving, setSaving] = createSignal(false);
  const [keepAction, setKeepAction] = createSignal("");

  // Markdown folder
  const [mdFolder, { refetch: refetchMdFolder }] = createResource<string | null>(markdownGetFolder);
  const [mdFolderInput, setMdFolderInput] = createSignal("");
  const [mdSaving, setMdSaving] = createSignal(false);

  createEffect(() => {
    const f = mdFolder();
    setMdFolderInput(f ?? "");
  });

  const [credentials, { refetch: refetchCredentials }] = createResource(keepCredentialsGet);
  const [keepClientId, setKeepClientId] = createSignal("");
  const [keepClientSecret, setKeepClientSecret] = createSignal("");
  const [credentialsSaving, setCredentialsSaving] = createSignal(false);

  // Connect flow state
  const [connectStatus, setConnectStatus] = createSignal<ConnectStatus>("idle");
  const [connectError, setConnectError] = createSignal("");

  createEffect(() => {
    const creds = credentials();
    if (creds) {
      setKeepClientId(creds.client_id ?? "");
      setKeepClientSecret(creds.client_secret ?? "");
    }
  });

  async function saveCredentials() {
    setCredentialsSaving(true);
    try {
      await keepCredentialsSet(
        keepClientId() || null,
        keepClientSecret() || null,
      );
      await refetchCredentials();
    } finally {
      setCredentialsSaving(false);
    }
  }

  async function save(partial: Partial<AppSettings>) {
    const current = settings();
    if (!current) return;
    const updated = { ...current, ...partial };
    mutate(updated);
    setSaving(true);
    try {
      await settingsSet(updated);
    } finally {
      setSaving(false);
    }
  }

  async function handleKeepRevoke() {
    setKeepAction("Desconectando…");
    try {
      await keepRevoke();
      setKeepAction("Desconectado.");
      await refetchKeepStatus();
    } catch (e) {
      setKeepAction(`Error: ${e}`);
    }
  }

  async function handleKeepConnect() {
    setConnectStatus("connecting");
    setConnectError("");

    let port: number | undefined;
    let unlisten: (() => void) | undefined;
    let timeoutId: ReturnType<typeof setTimeout> | undefined;

    async function cleanup() {
      clearTimeout(timeoutId);
      unlisten?.();
      if (port !== undefined) {
        try { await oauthCancel(port); } catch { /* already closed */ }
      }
    }

    try {
      // 1. Start the local OAuth callback server → get dynamic port
      port = await oauthStart();
      const redirectUri = `http://localhost:${port}`;

      // 2. Ask backend to build the auth URL (resolves credentials internally)
      const { auth_url, code_verifier, state: csrfState } = await keepStartAuth(redirectUri);

      // 3. Listen for the OAuth callback before opening the browser
      //    tauri-plugin-oauth emits "oauth://url" with the full callback URL
      const callbackPromise = new Promise<string>((resolve, reject) => {
        listen<string>("oauth://url", (event) => {
          resolve(event.payload);
        }).then((fn) => { unlisten = fn; });

        timeoutId = setTimeout(() => {
          reject(new Error("El flujo OAuth expiró. Cerrá esta ventana y volvé a intentarlo."));
        }, OAUTH_TIMEOUT_MS);
      });

      // 4. Open the browser with the auth URL
      await shellOpen(auth_url);

      // 5. Wait for the callback
      const callbackUrl = await callbackPromise;
      await cleanup();

      // 6. Parse code + state from the callback URL
      const params = new URL(callbackUrl).searchParams;
      const code = params.get("code");
      const stateReceived = params.get("state");
      const error = params.get("error");

      if (error) {
        throw new Error(`Google rechazó la autorización: ${error}`);
      }
      if (!code) {
        throw new Error("No se recibió el código de autorización de Google.");
      }
      if (stateReceived !== csrfState) {
        throw new Error("Error de seguridad: el parámetro state no coincide.");
      }

      // 7. Exchange code for tokens
      await keepCompleteAuth({ code, codeVerifier: code_verifier, redirectUri });

      // 8. Update UI
      await refetchKeepStatus();
      setConnectStatus("idle");
      setKeepAction("¡Conectado exitosamente!");
      triggerSync();

    } catch (e) {
      await cleanup();
      setConnectStatus("error");
      setConnectError(e instanceof Error ? e.message : String(e));
    }
  }

  async function saveMdFolder() {
    setMdSaving(true);
    try {
      await markdownSetFolder(mdFolderInput() || null);
      await refetchMdFolder();
    } finally {
      setMdSaving(false);
    }
  }

  // Obsidian vault
  const [obsidianVault, { refetch: refetchObsidianVault }] =
    createResource<string | null>(obsidianGetVault);
  const [obsidianVaultInput, setObsidianVaultInput] = createSignal("");
  const [obsidianSaving, setObsidianSaving] = createSignal(false);

  createEffect(() => {
    setObsidianVaultInput(obsidianVault() ?? "");
  });

  async function saveObsidianVault() {
    setObsidianSaving(true);
    try {
      await obsidianSetVault(obsidianVaultInput() || null);
      await refetchObsidianVault();
    } finally {
      setObsidianSaving(false);
    }
  }

  // ─── Notion auth ─────────────────────────────────────────────────────────────

  const [notionConnected, { refetch: refetchNotionStatus }] =
    createResource<boolean>(notionStatus);
  const [notionAction, setNotionAction] = createSignal("");
  const [notionConnectStatus, setNotionConnectStatus] = createSignal<ConnectStatus>("idle");
  const [notionConnectError, setNotionConnectError] = createSignal("");
  const [notionCreds, { refetch: refetchNotionCreds }] = createResource(notionCredentialsGet);
  const [notionClientId, setNotionClientId] = createSignal("");
  const [notionClientSecret, setNotionClientSecret] = createSignal("");
  const [notionParentId, setNotionParentId] = createSignal("");
  const [notionCredSaving, setNotionCredSaving] = createSignal(false);

  createEffect(() => {
    const c = notionCreds();
    if (c) {
      setNotionClientId(c.client_id ?? "");
      setNotionClientSecret(c.client_secret ?? "");
      setNotionParentId(c.parent_page_id ?? "");
    }
  });

  async function saveNotionCredentials() {
    setNotionCredSaving(true);
    try {
      await notionCredentialsSet(
        notionClientId() || null,
        notionClientSecret() || null,
        notionParentId() || null,
      );
      await refetchNotionCreds();
    } finally {
      setNotionCredSaving(false);
    }
  }

  async function handleNotionRevoke() {
    setNotionAction("Desconectando…");
    try {
      await notionRevoke();
      setNotionAction("Desconectado.");
      await refetchNotionStatus();
    } catch (e) {
      setNotionAction(`Error: ${e}`);
    }
  }

  async function handleNotionConnect() {
    setNotionConnectStatus("connecting");
    setNotionConnectError("");

    let port: number | undefined;
    let unlisten: (() => void) | undefined;
    let timeoutId: ReturnType<typeof setTimeout> | undefined;

    async function cleanup() {
      clearTimeout(timeoutId);
      unlisten?.();
      if (port !== undefined) {
        try { await oauthCancel(port); } catch { /* already closed */ }
      }
    }

    try {
      port = await oauthStart({ ports: [48542] });
      const redirectUri = `http://localhost:${port}`;

      const { auth_url, state: csrfState } = await notionStartAuth(redirectUri);

      const callbackPromise = new Promise<string>((resolve, reject) => {
        listen<string>("oauth://url", (event) => {
          resolve(event.payload);
        }).then((fn) => { unlisten = fn; });

        timeoutId = setTimeout(() => {
          reject(new Error("El flujo OAuth expiró. Cerrá esta ventana y volvé a intentarlo."));
        }, OAUTH_TIMEOUT_MS);
      });

      await shellOpen(auth_url);

      const callbackUrl = await callbackPromise;
      await cleanup();

      const params = new URL(callbackUrl).searchParams;
      const code = params.get("code");
      const stateReceived = params.get("state");
      const error = params.get("error");

      if (error) {
        throw new Error(`Notion rechazó la autorización: ${error}`);
      }
      if (!code) {
        throw new Error("No se recibió el código de autorización de Notion.");
      }
      if (stateReceived !== csrfState) {
        throw new Error("Error de seguridad: el parámetro state no coincide.");
      }

      await notionCompleteAuth({ code, redirectUri });

      await refetchNotionStatus();
      setNotionConnectStatus("idle");
      setNotionAction("¡Conectado exitosamente!");
      triggerSync();

    } catch (e) {
      await cleanup();
      setNotionConnectStatus("error");
      setNotionConnectError(e instanceof Error ? e.message : String(e));
    }
  }

  // ─── OneNote auth ────────────────────────────────────────────────────────────

  const [oneNoteConnected, { refetch: refetchOneNoteStatus }] =
    createResource<boolean>(oneNoteStatus);
  const [oneNoteAction, setOneNoteAction] = createSignal("");
  const [oneNoteConnectStatus, setOneNoteConnectStatus] = createSignal<ConnectStatus>("idle");
  const [oneNoteConnectError, setOneNoteConnectError] = createSignal("");
  const [oneNoteCredential, { refetch: refetchOneNoteCredential }] =
    createResource<string | null>(oneNoteCredentialsGet);
  const [oneNoteClientId, setOneNoteClientId] = createSignal("");
  const [oneNoteCredSaving, setOneNoteCredSaving] = createSignal(false);

  createEffect(() => {
    setOneNoteClientId(oneNoteCredential() ?? "");
  });

  async function saveOneNoteCredential() {
    setOneNoteCredSaving(true);
    try {
      await oneNoteCredentialsSet(oneNoteClientId() || null);
      await refetchOneNoteCredential();
    } finally {
      setOneNoteCredSaving(false);
    }
  }

  async function handleOneNoteRevoke() {
    setOneNoteAction("Desconectando…");
    try {
      await oneNoteRevoke();
      setOneNoteAction("Desconectado.");
      await refetchOneNoteStatus();
    } catch (e) {
      setOneNoteAction(`Error: ${e}`);
    }
  }

  async function handleOneNoteConnect() {
    setOneNoteConnectStatus("connecting");
    setOneNoteConnectError("");

    let port: number | undefined;
    let unlisten: (() => void) | undefined;
    let timeoutId: ReturnType<typeof setTimeout> | undefined;

    async function cleanup() {
      clearTimeout(timeoutId);
      unlisten?.();
      if (port !== undefined) {
        try { await oauthCancel(port); } catch { /* already closed */ }
      }
    }

    try {
      port = await oauthStart();
      const redirectUri = `http://localhost:${port}`;

      const { auth_url, code_verifier, state: csrfState } =
        await oneNoteStartAuth(redirectUri);

      const callbackPromise = new Promise<string>((resolve, reject) => {
        listen<string>("oauth://url", (event) => {
          resolve(event.payload);
        }).then((fn) => { unlisten = fn; });

        timeoutId = setTimeout(() => {
          reject(new Error("El flujo OAuth expiró. Cerrá esta ventana y volvé a intentarlo."));
        }, OAUTH_TIMEOUT_MS);
      });

      await shellOpen(auth_url);

      const callbackUrl = await callbackPromise;
      await cleanup();

      const params = new URL(callbackUrl).searchParams;
      const code = params.get("code");
      const stateReceived = params.get("state");
      const error = params.get("error");

      if (error) {
        throw new Error(`Microsoft rechazó la autorización: ${error}`);
      }
      if (!code) {
        throw new Error("No se recibió el código de autorización de Microsoft.");
      }
      if (stateReceived !== csrfState) {
        throw new Error("Error de seguridad: el parámetro state no coincide.");
      }

      await oneNoteCompleteAuth({ code, codeVerifier: code_verifier, redirectUri });

      await refetchOneNoteStatus();
      setOneNoteConnectStatus("idle");
      setOneNoteAction("¡Conectado exitosamente!");
      triggerSync();

    } catch (e) {
      await cleanup();
      setOneNoteConnectStatus("error");
      setOneNoteConnectError(e instanceof Error ? e.message : String(e));
    }
  }

  // ─── Export ───────────────────────────────────────────────────────────────

  const [exporting, setExporting] = createSignal(false);
  const [exportResult, setExportResult] = createSignal<{ path: string; count: number } | null>(null);
  const [exportError, setExportError] = createSignal("");

  async function handleExport(format: "json" | "markdown") {
    setExporting(true);
    setExportResult(null);
    setExportError("");
    try {
      const result = await notesExport(format);
      setExportResult(result);
    } catch (e) {
      setExportError(e instanceof Error ? e.message : String(e));
    } finally {
      setExporting(false);
    }
  }

  // Clean up if the component unmounts mid-flow (shouldn't happen, but just in case)
  onCleanup(() => {
    setConnectStatus("idle");
    setOneNoteConnectStatus("idle");
  });

  return (
    <div class="settings-panel">
      <h2 class="settings-panel__title">Configuración</h2>

      <div class="settings-panel__row">
        <label>Sync cada (minutos)</label>
        <input
          type="number"
          min="1"
          max="60"
          value={settings()?.sync_interval_minutes ?? 15}
          onChange={(e) =>
            save({ sync_interval_minutes: parseInt(e.currentTarget.value, 10) })
          }
        />
      </div>

      <div class="settings-panel__row">
        <label>Iniciar con Windows</label>
        <input
          type="checkbox"
          checked={settings()?.startup_with_windows ?? true}
          onChange={(e) => save({ startup_with_windows: e.currentTarget.checked })}
        />
      </div>

      <div class="settings-panel__row">
        <label>Minimizar al tray al cerrar</label>
        <input
          type="checkbox"
          checked={settings()?.show_in_tray ?? true}
          onChange={(e) => save({ show_in_tray: e.currentTarget.checked })}
        />
      </div>

      <div class="settings-panel__row">
        <label>Renderizar Markdown</label>
        <input
          type="checkbox"
          checked={settings()?.markdown_preview ?? false}
          onChange={(e) => save({ markdown_preview: e.currentTarget.checked })}
        />
      </div>

      <div class="settings-panel__row">
        <label>Tema</label>
        <select
          value={settings()?.theme ?? "system"}
          onChange={(e) =>
            save({ theme: e.currentTarget.value as AppSettings["theme"] })
          }
        >
          <option value="system">Sistema</option>
          <option value="light">Claro</option>
          <option value="dark">Oscuro</option>
        </select>
      </div>

      {/* Google Keep section */}
      <div class="settings-panel__section">
        <h3>Google Keep</h3>

        <Show when={keepConnected()} fallback={
          <div>
            <Show when={connectStatus() === "idle" || connectStatus() === "error"}>
              <button
                class="btn"
                onClick={handleKeepConnect}
                disabled={connectStatus() === "connecting"}
              >
                Conectar Google Keep
              </button>
            </Show>

            <Show when={connectStatus() === "connecting"}>
              <p class="settings-panel__note">
                Completá el paso en el browser que se acaba de abrir…
              </p>
              <button class="btn btn--secondary" onClick={async () => {
                setConnectStatus("idle");
              }}>
                Cancelar
              </button>
            </Show>

            <Show when={connectStatus() === "error"}>
              <p class="settings-panel__note settings-panel__note--error">
                {connectError()}
              </p>
            </Show>
          </div>
        }>
          <div class="settings-panel__row">
            <span style={{ color: "#4caf50" }}>✓ Conectado</span>
            <button class="btn" onClick={handleKeepRevoke}>Desconectar</button>
          </div>
        </Show>

        {keepAction() && <p class="settings-panel__note">{keepAction()}</p>}

        <details class="settings-panel__advanced">
          <summary>Credenciales personalizadas (opcional)</summary>
          <p class="settings-panel__note">
            Si tenés tu propio proyecto en Google Cloud Console, podés usar tus
            credenciales en lugar de las que vienen compiladas en la app.
          </p>
          <div class="settings-panel__row">
            <label>Client ID</label>
            <input
              type="text"
              placeholder="Compilado en la app"
              value={keepClientId()}
              onInput={(e) => setKeepClientId(e.currentTarget.value)}
            />
          </div>
          <div class="settings-panel__row">
            <label>Client Secret</label>
            <input
              type="password"
              placeholder="Compilado en la app"
              value={keepClientSecret()}
              onInput={(e) => setKeepClientSecret(e.currentTarget.value)}
            />
          </div>
          <div class="settings-panel__row">
            <button class="btn" onClick={saveCredentials} disabled={credentialsSaving()}>
              {credentialsSaving() ? "Guardando…" : "Guardar credenciales"}
            </button>
          </div>
        </details>
      </div>

      {/* Markdown folder section */}
      <div class="settings-panel__section">
        <h3>Carpeta Markdown</h3>
        <p class="settings-panel__note">
          Pegá la ruta a una carpeta con archivos <code>.md</code>. Cada archivo
          se importa como una nota.
        </p>
        <div class="settings-panel__row settings-panel__row--column">
          <input
            type="text"
            class="settings-panel__text-input"
            placeholder="Ej: C:\Users\vos\Documents\notas"
            value={mdFolderInput()}
            onInput={(e) => setMdFolderInput(e.currentTarget.value)}
          />
          <div style={{ display: "flex", gap: "8px", "margin-top": "8px" }}>
            <button class="btn" onClick={saveMdFolder} disabled={mdSaving()}>
              {mdSaving() ? "Guardando…" : "Guardar carpeta"}
            </button>
            <Show when={mdFolder()}>
              <button class="btn" onClick={() => { setMdFolderInput(""); saveMdFolder(); }}>
                Quitar
              </button>
            </Show>
          </div>
        </div>
      </div>

      {/* Notion section */}
      <div class="settings-panel__section">
        <h3>Notion</h3>

        <Show when={notionConnected()} fallback={
          <div>
            <Show when={notionConnectStatus() === "idle" || notionConnectStatus() === "error"}>
              <button
                class="btn"
                onClick={handleNotionConnect}
                disabled={notionConnectStatus() === "connecting"}
              >
                Conectar Notion
              </button>
            </Show>
            <Show when={notionConnectStatus() === "connecting"}>
              <p class="settings-panel__note">Completá el paso en el browser…</p>
              <button class="btn btn--secondary" onClick={() => setNotionConnectStatus("idle")}>
                Cancelar
              </button>
            </Show>
            <Show when={notionConnectStatus() === "error"}>
              <p class="settings-panel__note settings-panel__note--error">{notionConnectError()}</p>
            </Show>
          </div>
        }>
          <div class="settings-panel__row">
            <span style={{ color: "#4caf50" }}>✓ Conectado</span>
            <button class="btn" onClick={handleNotionRevoke}>Desconectar</button>
          </div>
        </Show>

        {notionAction() && <p class="settings-panel__note">{notionAction()}</p>}

        <details class="settings-panel__advanced">
          <summary>Credenciales de integración (requeridas)</summary>
          <p class="settings-panel__note">
            Registrá una integración pública en{" "}
            <strong>notion.so/profile/integrations</strong> y pegá las credenciales acá.
            El <em>Parent Page ID</em> es la página de Notion donde se crearán las notas nuevas.
          </p>
          <div class="settings-panel__row">
            <label>OAuth Client ID</label>
            <input type="text" placeholder="Compilado en la app" value={notionClientId()}
              onInput={(e) => setNotionClientId(e.currentTarget.value)} />
          </div>
          <div class="settings-panel__row">
            <label>OAuth Client Secret</label>
            <input type="password" placeholder="Compilado en la app" value={notionClientSecret()}
              onInput={(e) => setNotionClientSecret(e.currentTarget.value)} />
          </div>
          <div class="settings-panel__row">
            <label>Parent Page ID</label>
            <input type="text" placeholder="ID de la página destino" value={notionParentId()}
              onInput={(e) => setNotionParentId(e.currentTarget.value)} />
          </div>
          <div class="settings-panel__row">
            <button class="btn" onClick={saveNotionCredentials} disabled={notionCredSaving()}>
              {notionCredSaving() ? "Guardando…" : "Guardar credenciales"}
            </button>
          </div>
        </details>
      </div>

      {/* Obsidian vault section */}
      <div class="settings-panel__section">
        <h3>Obsidian Vault</h3>
        <p class="settings-panel__note">
          Pegá la ruta a tu vault de Obsidian. Los archivos <code>.md</code> se
          importan recursivamente — los backlinks <code>[[…]]</code> se convierten
          a texto y los <code>#tags</code> inline se extraen como labels.
        </p>
        <div class="settings-panel__row settings-panel__row--column">
          <input
            type="text"
            class="settings-panel__text-input"
            placeholder="Ej: C:\Users\vos\Documents\MyVault"
            value={obsidianVaultInput()}
            onInput={(e) => setObsidianVaultInput(e.currentTarget.value)}
          />
          <div style={{ display: "flex", gap: "8px", "margin-top": "8px" }}>
            <button class="btn" onClick={saveObsidianVault} disabled={obsidianSaving()}>
              {obsidianSaving() ? "Guardando…" : "Guardar vault"}
            </button>
            <Show when={obsidianVault()}>
              <button class="btn" onClick={() => { setObsidianVaultInput(""); saveObsidianVault(); }}>
                Quitar
              </button>
            </Show>
          </div>
        </div>
      </div>

      {/* Microsoft OneNote section */}
      <div class="settings-panel__section">
        <h3>Microsoft OneNote</h3>

        <Show when={oneNoteConnected()} fallback={
          <div>
            <Show when={oneNoteConnectStatus() === "idle" || oneNoteConnectStatus() === "error"}>
              <button
                class="btn"
                onClick={handleOneNoteConnect}
                disabled={oneNoteConnectStatus() === "connecting"}
              >
                Conectar OneNote
              </button>
            </Show>

            <Show when={oneNoteConnectStatus() === "connecting"}>
              <p class="settings-panel__note">
                Completá el paso en el browser que se acaba de abrir…
              </p>
              <button class="btn btn--secondary" onClick={() => setOneNoteConnectStatus("idle")}>
                Cancelar
              </button>
            </Show>

            <Show when={oneNoteConnectStatus() === "error"}>
              <p class="settings-panel__note settings-panel__note--error">
                {oneNoteConnectError()}
              </p>
            </Show>
          </div>
        }>
          <div class="settings-panel__row">
            <span style={{ color: "#4caf50" }}>✓ Conectado</span>
            <button class="btn" onClick={handleOneNoteRevoke}>Desconectar</button>
          </div>
        </Show>

        {oneNoteAction() && <p class="settings-panel__note">{oneNoteAction()}</p>}

        <details class="settings-panel__advanced">
          <summary>Azure App ID personalizado (opcional)</summary>
          <p class="settings-panel__note">
            Si registraste tu propia app en Azure Portal, podés usar tu propio
            Application (client) ID.
          </p>
          <div class="settings-panel__row">
            <label>Application (Client) ID</label>
            <input
              type="text"
              placeholder="Compilado en la app"
              value={oneNoteClientId()}
              onInput={(e) => setOneNoteClientId(e.currentTarget.value)}
            />
          </div>
          <div class="settings-panel__row">
            <button class="btn" onClick={saveOneNoteCredential} disabled={oneNoteCredSaving()}>
              {oneNoteCredSaving() ? "Guardando…" : "Guardar App ID"}
            </button>
          </div>
        </details>
      </div>

      {/* Export section */}
      <div class="settings-panel__section">
        <h3>Exportar notas</h3>
        <p class="settings-panel__note">
          Las notas se guardan en <code>Documentos\Skeepy Export\</code>.
        </p>
        <div class="settings-panel__row">
          <button class="btn" onClick={() => handleExport("json")} disabled={exporting()}>
            {exporting() ? "Exportando…" : "Exportar JSON"}
          </button>
          <button class="btn" onClick={() => handleExport("markdown")} disabled={exporting()}>
            {exporting() ? "Exportando…" : "Exportar Markdown"}
          </button>
        </div>
        <Show when={exportResult()}>
          <p class="settings-panel__note" style={{ color: "#4caf50" }}>
            ✓ {exportResult()!.count} notas exportadas → <code>{exportResult()!.path}</code>
          </p>
        </Show>
        <Show when={exportError()}>
          <p class="settings-panel__note settings-panel__note--error">{exportError()}</p>
        </Show>
      </div>

      {/* Labels section */}
      <div class="settings-panel__section">
        <h3>Labels / Tags</h3>
        <p class="settings-panel__note">
          Las labels de providers remotos (Keep, OneNote) son de solo lectura —
          administralas desde la app original.
        </p>
        <LabelsPanel />
      </div>

      {/* Active providers */}
      <div class="settings-panel__section">
        <h3>Providers activos</h3>
        <ProviderStatusPanel />
      </div>

      {/* Data directory info */}
      <Show when={dataDir()}>
        <div class="settings-panel__section">
          <h3>Archivos de datos</h3>
          <p class="settings-panel__note">
            Poné tu <code>notes.json</code> en:<br />
            <code class="settings-panel__path">{dataDir()}</code>
          </p>
        </div>
      </Show>

      {/* About + updates */}
      <div class="settings-panel__section">
        <h3>Acerca de</h3>
        <div class="settings-panel__row">
          <span>Versión</span>
          <span class="settings-panel__version">{appVersion() ?? "—"}</span>
        </div>
        <div class="settings-panel__row">
          <button
            class="btn"
            onClick={handleCheckForUpdates}
            disabled={checkState() === "checking" || installing()}
          >
            {checkState() === "checking" ? "Buscando…" : "Buscar actualizaciones"}
          </button>
        </div>
        <Show when={checkState() === "uptodate"}>
          <p class="settings-panel__note" style={{ color: "#4caf50" }}>
            ✓ Ya tenés la última versión.
          </p>
        </Show>
        <Show when={checkState() === "available" && updateInfo()}>
          <p class="settings-panel__note">
            Nueva versión <strong>{updateInfo()!.version}</strong> disponible.
          </p>
          <button
            class="btn btn--primary"
            onClick={handleInstallUpdate}
            disabled={installing()}
          >
            {installing() ? "Instalando…" : "Instalar y reiniciar"}
          </button>
        </Show>
        <Show when={checkState() === "error"}>
          <p class="settings-panel__note settings-panel__note--error">{updateError()}</p>
        </Show>
      </div>

      {saving() && <p class="settings-panel__saving">Guardando…</p>}
    </div>
  );
};
