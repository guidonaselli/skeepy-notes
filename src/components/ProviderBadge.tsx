import type { Component } from "solid-js";

interface Props {
  providerId: string;
}

const PROVIDER_LABELS: Record<string, string> = {
  local: "local",
  keep: "Keep",
};

const PROVIDER_COLORS: Record<string, string> = {
  local: "#4a9eff",
  keep: "#fbbc04",
};

export const ProviderBadge: Component<Props> = (props) => {
  const label = () => PROVIDER_LABELS[props.providerId] ?? props.providerId;
  const color = () => PROVIDER_COLORS[props.providerId] ?? "#888";

  return (
    <span
      class="provider-badge"
      style={{ "background-color": color() }}
      title={`Provider: ${props.providerId}`}
    >
      {label()}
    </span>
  );
};
