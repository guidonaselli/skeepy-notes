import { type Component, type JSX } from "solid-js";
import {
  IconNote,
  IconCloud,
  IconNotebook,
  IconDatabase,
  IconFolderOpen,
  IconTreeStructure,
  IconMonitor,
} from "./Icons";

interface ProviderCard {
  icon: () => JSX.Element;
  name: string;
  description: string;
  action: "settings" | "active";
  actionLabel: string;
}

const PROVIDERS: ProviderCard[] = [
  {
    icon: () => <IconNote size={22} />,
    name: "Notas locales",
    description: "Notas propias de Skeepy, guardadas en tu PC. Ya está activo.",
    action: "active",
    actionLabel: "Activo",
  },
  {
    icon: () => <IconCloud size={22} />,
    name: "Google Keep",
    description: "Sincronizá tus notas de Google Keep. Requiere una cuenta de Google.",
    action: "settings",
    actionLabel: "Conectar",
  },
  {
    icon: () => <IconNotebook size={22} />,
    name: "Microsoft OneNote",
    description: "Accedé a tus cuadernos de OneNote con tu cuenta Microsoft.",
    action: "settings",
    actionLabel: "Conectar",
  },
  {
    icon: () => <IconDatabase size={22} />,
    name: "Notion",
    description: "Importá y editá páginas de Notion. Requiere credenciales propias.",
    action: "settings",
    actionLabel: "Conectar",
  },
  {
    icon: () => <IconFolderOpen size={22} />,
    name: "Carpeta Markdown",
    description: "Cualquier carpeta con archivos .md en tu PC se convierte en notas.",
    action: "settings",
    actionLabel: "Configurar",
  },
  {
    icon: () => <IconTreeStructure size={22} />,
    name: "Obsidian Vault",
    description: "Importá tu vault de Obsidian con backlinks y tags inline.",
    action: "settings",
    actionLabel: "Configurar",
  },
  {
    icon: () => <IconMonitor size={22} />,
    name: "Windows Sticky Notes",
    description: "Importa las notas del sistema automáticamente. Solo Windows.",
    action: "active",
    actionLabel: "Auto-detectado",
  },
];

interface Props {
  onOpenSettings: () => void;
  onCreateNote: () => void;
}

export const WelcomeScreen: Component<Props> = (props) => {
  return (
    <div class="welcome">
      <div class="welcome__hero">
        <div class="welcome__logo">
          <IconNote size={48} />
        </div>
        <h1 class="welcome__title">Bienvenido a Skeepy</h1>
        <p class="welcome__subtitle">
          Conectá tus fuentes de notas o creá tu primera nota local.
        </p>
      </div>

      <div class="welcome__grid">
        {PROVIDERS.map((p) => (
          <div class="welcome__card">
            <span class="welcome__card-icon">{p.icon()}</span>
            <div class="welcome__card-body">
              <strong class="welcome__card-name">{p.name}</strong>
              <span class="welcome__card-desc">{p.description}</span>
            </div>
            {p.action === "active" ? (
              <span class="welcome__badge">{p.actionLabel}</span>
            ) : (
              <button class="btn welcome__card-btn" onClick={props.onOpenSettings}>
                {p.actionLabel}
              </button>
            )}
          </div>
        ))}
      </div>

      <div class="welcome__footer">
        <button class="btn btn--primary welcome__create-btn" onClick={props.onCreateNote}>
          Crear primera nota
        </button>
        <p class="welcome__footer-note">
          También podés sincronizar con el boton de refresh o abrir Configuracion con el engranaje en la barra superior.
        </p>
      </div>
    </div>
  );
};
