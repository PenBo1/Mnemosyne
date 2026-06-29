// ── Provider / Model ───────────────────────────────────────

export interface ModelInfo {
  id: string;
  provider: string;
  name: string;
  context_window: number;
  supports_tools: boolean;
  supports_streaming: boolean;
}

export interface ProviderInfo {
  name: string;
  models: ModelInfo[];
}

export interface ProviderConfig {
  api_key: string;
  base_url: string | null;
}

export interface ProviderSettings {
  default_provider: string;
  default_model: string;
  configs: Record<string, ProviderConfig>;
}
