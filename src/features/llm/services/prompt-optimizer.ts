/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Prompt Optimizer Service - 提示词优化服务
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * 封装 prompt_optimize IPC 调用，为组件提供清晰的 API。
 */

import { ipc } from "@/services/ipc";

// ── 类型定义 ────────────────────────────────────────────────────────────────

/**
 * 提示词优化响应
 */
export interface PromptOptimizeResponse {
  /** 优化后的提示词 */
  optimized_prompt: string;
}

// ── 服务函数 ────────────────────────────────────────────────────────────────

/**
 * 优化提示词
 *
 * 通过 IPC 调用 Rust 后端的 PromptOptimizer 进行提示词优化。
 *
 * @param prompt - 原始提示词
 * @returns 优化后的提示词
 * @throws 当优化失败时抛出错误
 *
 * @example
 * ```typescript
 * const optimized = await optimizePrompt("写一个故事");
 * console.log(optimized); // 优化后的提示词
 * ```
 */
export async function optimizePrompt(prompt: string): Promise<string> {
  const response = await ipc<PromptOptimizeResponse>("prompt_optimize", {
    prompt,
  });
  return response.optimized_prompt;
}