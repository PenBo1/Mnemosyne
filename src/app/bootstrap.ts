//! 应用初始化
//!
//! 初始化顺序：
//! 1. 加载配置（主题、语言、模型）
//! 2. 连接 Rust Kernel
//! 3. 初始化 Agent Controller
//! 4. 加载 Workspace 状态
//! 5. 启动 UI

import { ipc } from "@/services/ipc";

export interface AppConfig {
  logLevel: string;
}

export interface BootstrapResult {
  config: AppConfig;
  initialized: boolean;
}

export async function bootstrap(): Promise<BootstrapResult> {
  const logLevel = await ipc<string>("get_log_level");

  return {
    config: {
      logLevel,
    },
    initialized: true,
  };
}