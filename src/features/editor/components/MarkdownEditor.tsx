/**
 * ═══════════════════════════════════════════════════════════════════════════
 * MarkdownEditor - 分栏 Markdown 编辑器组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useMemo, useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  BoldIcon,
  ItalicIcon,
  Heading1Icon,
  Heading2Icon,
  Heading3Icon,
  ListIcon,
  ListOrderedIcon,
  CodeIcon,
  LinkIcon,
  TableIcon,
  EditIcon,
  EyeIcon,
  ColumnsIcon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { CodeEditor } from "@/features/editor/components/CodeEditor";
import { MarkdownRenderer } from "@/features/chat/components/markdown-renderer";
import type { MarkdownViewMode } from "@/features/editor/types";
import { cn } from "@/lib/utils";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface MarkdownEditorProps {
  value: string;
  onChange?: (value: string) => void;
  onSave?: () => void;
  readOnly?: boolean;
  className?: string;
}

// ── 常量配置 ────────────────────────────────────────────────────────────────

const STORAGE_KEY = "mnemosyne:markdown-view-mode";

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 从 localStorage 读取上次视图模式
 */
function loadViewMode(): MarkdownViewMode {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "split" || saved === "edit" || saved === "preview") {
      return saved;
    }
  } catch {
    // localStorage 不可用时忽略
  }
  return "split";
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 分栏 Markdown 编辑器，支持编辑、预览和分栏三种模式
 */
export function MarkdownEditor({
  value,
  onChange,
  onSave,
  readOnly = false,
  className,
}: MarkdownEditorProps) {
  const { t } = useI18n();
  const [viewMode, setViewMode] = useState<MarkdownViewMode>(loadViewMode);
  // 防抖后的预览内容（300ms）
  const [previewContent, setPreviewContent] = useState(value);

  // 视图模式持久化
  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, viewMode);
    } catch {
      // 忽略写入失败
    }
  }, [viewMode]);

  // 防抖同步预览（300ms）
  useEffect(() => {
    const timer = setTimeout(() => {
      setPreviewContent(value);
    }, 300);
    return () => clearTimeout(timer);
  }, [value]);

  // ── 工具栏操作 ────────────────────────────────────────────────────────────

  /**
   * 在编辑器内容中插入 Markdown 语法
   */
  const insertSyntax = useCallback(
    (before: string, after: string = "", placeholder: string = "") => {
      if (!onChange) return;
      // 简单实现：在末尾插入语法
      const insertion = `${before}${placeholder}${after}`;
      onChange(value + (value && !value.endsWith("\n") ? "\n" : "") + insertion);
    },
    [onChange, value],
  );

  // ── 工具栏渲染 ────────────────────────────────────────────────────────────

  const toolbar = useMemo(
    () => (
      <div className="flex items-center gap-1 border-b border-border px-2 py-1">
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() => insertSyntax("**", "**", t.editor.bold)}
          title={t.editor.bold}
          disabled={readOnly}
        >
          <BoldIcon className="size-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() => insertSyntax("*", "*", t.editor.italic)}
          title={t.editor.italic}
          disabled={readOnly}
        >
          <ItalicIcon className="size-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() => insertSyntax("# ", "", t.editor.heading)}
          title="H1"
          disabled={readOnly}
        >
          <Heading1Icon className="size-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() => insertSyntax("## ", "", t.editor.heading)}
          title="H2"
          disabled={readOnly}
        >
          <Heading2Icon className="size-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() => insertSyntax("### ", "", t.editor.heading)}
          title="H3"
          disabled={readOnly}
        >
          <Heading3Icon className="size-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() => insertSyntax("- ", "", t.editor.list)}
          title={t.editor.list}
          disabled={readOnly}
        >
          <ListIcon className="size-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() => insertSyntax("1. ", "", t.editor.list)}
          title={t.editor.list}
          disabled={readOnly}
        >
          <ListOrderedIcon className="size-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() => insertSyntax("\n```\n", "\n```\n", t.editor.code)}
          title={t.editor.code}
          disabled={readOnly}
        >
          <CodeIcon className="size-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() => insertSyntax("[", "](url)", t.editor.link)}
          title={t.editor.link}
          disabled={readOnly}
        >
          <LinkIcon className="size-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() =>
            insertSyntax(
              `\n| ${t.editor.tableTemplate.column1} | ${t.editor.tableTemplate.column2} | ${t.editor.tableTemplate.column3} |\n|---|---|---|\n| | | |\n`,
              "",
              "",
            )
          }
          title={t.editor.table}
          disabled={readOnly}
        >
          <TableIcon className="size-4" />
        </Button>

        <div className="ml-auto flex items-center gap-1">
          {/* 视图模式切换 */}
          <Button
            variant={viewMode === "edit" ? "default" : "ghost"}
            size="icon-xs"
            onClick={() => setViewMode("edit")}
            title={t.editor.markdownEdit}
          >
            <EditIcon className="size-4" />
          </Button>
          <Button
            variant={viewMode === "split" ? "default" : "ghost"}
            size="icon-xs"
            onClick={() => setViewMode("split")}
            title={t.editor.markdownSplit}
          >
            <ColumnsIcon className="size-4" />
          </Button>
          <Button
            variant={viewMode === "preview" ? "default" : "ghost"}
            size="icon-xs"
            onClick={() => setViewMode("preview")}
            title={t.editor.markdownPreview}
          >
            <EyeIcon className="size-4" />
          </Button>
        </div>
      </div>
    ),
    [insertSyntax, readOnly, t, viewMode],
  );

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <div className={cn("flex h-full flex-col", className)}>
      {toolbar}
      <div className="flex flex-1 min-h-0">
        {/* 编辑区 */}
        {viewMode !== "preview" && (
          <div className={cn("min-h-0", viewMode === "split" ? "w-1/2 border-r border-border" : "w-full")}>
            <CodeEditor
              value={value}
              language="markdown"
              readOnly={readOnly}
              onChange={onChange}
              onSave={onSave}
            />
          </div>
        )}
        {/* 预览区 */}
        {viewMode !== "edit" && (
          <div className={cn("min-h-0", viewMode === "split" ? "w-1/2" : "w-full")}>
            <ScrollArea className="h-full">
              <div className="p-4">
                <MarkdownRenderer content={previewContent} />
              </div>
            </ScrollArea>
          </div>
        )}
      </div>
    </div>
  );
}