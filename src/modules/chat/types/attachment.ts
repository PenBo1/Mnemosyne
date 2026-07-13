// ── Chat Attachments ───────────────────────────────────────
//
// 附件类型定义：用户通过操作栏选择添加到 AI agent 上下文的内容。
// 与 src-tauri/src/core/agent/attachments.rs 的 AttachmentSpec 对齐。
//
// 设计原则：
// - 前端只传「引用」（路径/wiki_id/章节号），由 Rust 端读取实际内容
// - 路径不暴露给 LLM（信任边界要求），Rust 端只把文件内容注入上下文
// - file_path 是用户通过文件选择器选取的任意路径，Rust 端校验安全性后读取

/**
 * 附件类型：
 * - file: 任意文件（Rust 端读取文本内容注入上下文）
 * - wiki: Wiki 条目（按 entry_id 从 DB 读取）
 * - chapter: 章节文件（按 workspace + chapter_number 读取）
 * - text: 用户手动输入的文本片段
 */
export type AttachmentKind = "file" | "wiki" | "chapter" | "text";

export interface AttachmentSpec {
  kind: AttachmentKind;
  /** file: 文件绝对路径；wiki: entry_id；chapter: chapter_number（字符串形式）；text: 无 */
  ref: string;
  /** 显示名称（前端展示用，不传给 LLM） */
  label: string;
  /** text 类型时的文本内容；其他类型为空，由 Rust 端读取 */
  content?: string;
}

export interface SendMessageParams {
  sessionId: string;
  content: string;
  attachments?: AttachmentSpec[];
}
