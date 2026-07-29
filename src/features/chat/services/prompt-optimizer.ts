/**
 * ═══════════════════════════════════════════════════════════════════════════
 * PromptOptimizer - 提示词优化服务
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { invoke } from "@tauri-apps/api/core";
import type { IpcResponse } from "@/services/ipc";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface OptimizeResponse {
  optimized_prompt: string;
}

// ── 优化系统提示词 ────────────────────────────────────────────────────────────

const OPTIMIZATION_SYSTEM_PROMPT = `你是一个提示词优化专家。你的任务是帮助用户优化他们的提示词，使其更加清晰、具体、易于 AI 理解和执行。

优化原则：
1. 保持用户的原始意图不变
2. 使提示词更加具体和清晰
3. 添加必要的上下文信息
4. 使用结构化的表达方式
5. 移除冗余和模糊的表述

请直接返回优化后的提示词，不要添加任何解释或说明。`;

// ── 主函数 ──────────────────────────────────────────────────────────────────

/**
 * 优化用户提示词
 * @param prompt 用户原始提示词
 * @param sessionId 当前会话 ID（可选，用于获取模型配置）
 * @returns 优化后的提示词，如果失败则返回 null
 */
export async function optimizePrompt(
  prompt: string,
  sessionId?: string
): Promise<string | null> {
  try {
    const response = await invoke<IpcResponse<OptimizeResponse>>(
      "prompt_optimize",
      {
        prompt,
        sessionId: sessionId ?? null,
        systemPrompt: OPTIMIZATION_SYSTEM_PROMPT,
      }
    );

    if (response.status === 0 && response.data) {
      return response.data.optimized_prompt;
    }

    console.error("Prompt optimization failed:", response.message);
    return null;
  } catch (error) {
    console.error("Prompt optimization error:", error);
    return null;
  }
}

/**
 * 使用简单 LLM 调用优化提示词（备用方案）
 * 直接通过 fetch 调用 LLM API
 */
export async function optimizePromptDirect(
  prompt: string,
  modelConfig: {
    provider: string;
    model: string;
    api_key: string;
    base_url: string;
  }
): Promise<string | null> {
  const { provider, model, api_key, base_url } = modelConfig;

  try {
    // 根据提供商选择不同的 API 格式
    if (provider === "anthropic") {
      const response = await fetch(`${base_url}/v1/messages`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-api-key": api_key,
          "anthropic-version": "2023-06-01",
        },
        body: JSON.stringify({
          model,
          max_tokens: 2048,
          system: OPTIMIZATION_SYSTEM_PROMPT,
          messages: [
            {
              role: "user",
              content: prompt,
            },
          ],
        }),
      });

      if (!response.ok) {
        throw new Error(`API request failed: ${response.status}`);
      }

      const data = await response.json();
      const content = data.content?.[0]?.text;
      return content ?? null;
    } else {
      // OpenAI 兼容格式
      const response = await fetch(`${base_url}/v1/chat/completions`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${api_key}`,
        },
        body: JSON.stringify({
          model,
          max_tokens: 2048,
          messages: [
            {
              role: "system",
              content: OPTIMIZATION_SYSTEM_PROMPT,
            },
            {
              role: "user",
              content: prompt,
            },
          ],
        }),
      });

      if (!response.ok) {
        throw new Error(`API request failed: ${response.status}`);
      }

      const data = await response.json();
      const content = data.choices?.[0]?.message?.content;
      return content ?? null;
    }
  } catch (error) {
    console.error("Direct prompt optimization error:", error);
    return null;
  }
}