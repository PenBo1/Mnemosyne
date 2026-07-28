// 编辑器功能模块类型定义

/** 编辑器支持的语言 */
export type EditorLanguage =
  | "typescript"
  | "javascript"
  | "jsx"
  | "tsx"
  | "rust"
  | "json"
  | "markdown"
  | "html"
  | "css"
  | "python"
  | "toml"
  | "yaml"
  | "text";

/** 文件读取返回结构（对应 Rust EditorFileContent） */
export interface EditorFileContent {
  content: string;
  language: string;
  size: number;
  path: string;
}

/** 最近文件项（对应 Rust RecentFile） */
export interface RecentFile {
  path: string;
  name: string;
  extension: string | null;
  modifiedAt: number;
}

/** 编辑器 Tab 状态 */
export interface EditorTab {
  id: string;
  path: string;
  name: string;
  language: string;
  content: string;
  /** 原始内容，用于判断是否有未保存改动 */
  originalContent: string;
  isDirty: boolean;
  readOnly: boolean;
}

/** Diff 视图模式 */
export type DiffMode = "inline" | "split";

/** Markdown 编辑器视图模式 */
export type MarkdownViewMode = "split" | "edit" | "preview";
