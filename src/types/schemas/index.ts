/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Zod Schema 导出 - 隔离 zod 依赖
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * Schema 文件使用 zod 进行运行时校验，独立 barrel 避免污染主 @/types。
 * 领域特定 schema 已移动到对应的 feature 模块：
 * - book-rules, genre-profile → features/story/types/
 * - input-governance, length-governance → features/agent/types/
 */

// ── 上下文压缩 Schema ────────────────────────────────────────────────────────

export * from "../context-compression";