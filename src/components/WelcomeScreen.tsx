import { type Component } from "solid-js";

interface ProviderCard {
  icon: string;
  name: string;
  description: string;
  action: "settings" | "active";
  actionLabel: string;
}

const PROVIDERS: ProviderCard[] = [
  {
    icon: "📋",
    name: "Notas locales",
    description: "Notas propias de Skeepy, guardadas en tu PC. Ya está activo.",
    action: "active",
    actionLabel: "Activo",
  },
  {
    icon: "🟡",
    name: "Google Keep",
    description: "Sincronizá tus notas de Google Keep. Requiere una cuenta de Google.",
    action: "settings",
    actionLabel: "Conectar",
  },
  {
    icon: "🔵",
    name: "Microsoft OneNote",
    description: "Accedé a tus cuadernos de OneNote con tu cuenta Microsoft.",
    action: "settings",
    actionLabel: "Conectar",
  },
  {
    icon: "⬛",
    name: "Notion",
    description: "Importá y editá páginas de Notion. Requiere credenciales de integración propias.",
    action: "settings",
    actionLabel: "Conectar",
  },
  {
    icon: "📁",
    name: "Carpeta Markdown",
    description: "Cualquier carpeta con archivos .md en tu PC se convierte en notas.",
    action: "settings",
    actionLabel: "Configurar",
  },
  {
    icon: "🔮",
    name: "Obsidian Vault",
    description: "Importá tu vault de Obsidian con backlinks [[…]] y #tags inline.",
    action: "settings",
    actionLabel: "Configurar",
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
        <div class="welcome__logo">🗒️</div>
        <h1 class="welcome__title">Bienvenido a Skeepy</h1>
        <p class="welcome__subtitle">
          Conectá tus fuentes de notas o creá tu primera nota local.
        </p>
      </div>

      <div class="welcome__grid">
        {PROVIDERS.map((p) => (
          <div class="welcome__card">
            <span class="welcome__card-icon">{p.icon}</span>
            <div class="welcome__card-body">
              <strong class="welcome__card-name">{p.name}</strong>
              <span class="welcome__card-desc">{p.description}</span>
            </div>
            {p.action === "active" ? (
              <span class="welcome__badge">✓ Activo</span>
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
          ＋ Crear primera nota
        </button>
        <p class="welcome__footer-note">
          También podés sincronizar con ↻ o abrir Configuración con ⚙ en la barra superior.
        </p>
      </div>
    </div>
  );
};
