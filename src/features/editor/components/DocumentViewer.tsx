/**
 * ═══════════════════════════════════════════════════════════════════════════
 * DocumentViewer - 只读文档查看器组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { CodeEditor } from "./CodeEditor";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface DocumentViewerProps {
  value: string;
  language: string;
  className?: string;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 只读文档查看器，强制 readOnly=true，复用 CodeEditor 内核
 */
export function DocumentViewer({ value, language, className }: DocumentViewerProps) {
  return (
    <CodeEditor
      value={value}
      language={language}
      readOnly
      dark
      className={className}
    />
  );
}