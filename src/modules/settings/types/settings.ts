/** AI 模型配置 */
export interface AiModelConfig {
  id: string;
  name: string;
  provider: string;
  model: string;
  api_key: string;
  base_url: string;
}

/** 日志级别 */
export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";

/** 应用设置 */
export interface AppSettings {
  ui: {
    theme: "light" | "dark" | "system";
    locale: "en" | "zh";
    notifications: boolean;
  };
  system: {
    log_level: LogLevel;
  };
  ai: {
    models: AiModelConfig[];
    active_model_id: string | null;
  };
}
