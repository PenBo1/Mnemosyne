// CodeMirror 语言扩展映射
//
// 所有语言扩展改为动态 import：编辑器初始 bundle 不再包含 7 个 lang-* 包，
// 仅在用户实际打开对应语言文件时按需下载。markdown 语言扩展未被 CodeMirror
// 核心依赖（react-markdown 走自己的渲染路径），可安全动态加载。
import type { Extension } from "@codemirror/state";

type LanguageLoader = () => Promise<Extension>;

/** 语言字符串 → CodeMirror 语言扩展动态加载器映射 */
const LANGUAGE_LOADERS: Record<string, LanguageLoader> = {
  typescript: async () => (await import("@codemirror/lang-javascript")).javascript({ typescript: true }),
  javascript: async () => (await import("@codemirror/lang-javascript")).javascript(),
  tsx: async () => (await import("@codemirror/lang-javascript")).javascript({ jsx: true, typescript: true }),
  jsx: async () => (await import("@codemirror/lang-javascript")).javascript({ jsx: true }),
  rust: async () => (await import("@codemirror/lang-rust")).rust(),
  json: async () => (await import("@codemirror/lang-json")).json(),
  markdown: async () => (await import("@codemirror/lang-markdown")).markdown(),
  html: async () => (await import("@codemirror/lang-html")).html(),
  css: async () => (await import("@codemirror/lang-css")).css(),
  python: async () => (await import("@codemirror/lang-python")).python(),
};

/**
 * 根据语言字符串获取 CodeMirror 语言扩展。
 *
 * 异步动态加载：首次调用某语言时会触发对应 lang-* 包的网络请求，
 * 调用方需 await 后再 reconfigure 到 EditorView。
 */
export async function getLanguageExtension(language: string): Promise<Extension[]> {
  const loader = LANGUAGE_LOADERS[language];
  return loader ? [await loader()] : [];
}
