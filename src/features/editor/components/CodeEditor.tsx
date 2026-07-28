/**
 * ═══════════════════════════════════════════════════════════════════════════
 * CodeEditor - 基于 CodeMirror 6 的代码编辑器组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useRef } from "react";
import { EditorState, Compartment } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection, dropCursor, rectangularSelection, crosshairCursor } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, foldGutter, indentOnInput, syntaxHighlighting, defaultHighlightStyle, indentUnit } from "@codemirror/language";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { closeBrackets, closeBracketsKeymap, autocompletion, completionKeymap } from "@codemirror/autocomplete";
import { oneDark } from "@codemirror/theme-one-dark";
import { getLanguageExtension } from "@/features/editor/services/languages";
import { cn } from "@/lib/utils";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface CodeEditorProps {
  /** 编辑器内容 */
  value: string;
  /** 语言标识（typescript/javascript/rust/json/markdown 等） */
  language: string;
  /** 是否只读 */
  readOnly?: boolean;
  /** 内容变化回调 */
  onChange?: (value: string) => void;
  /** 保存回调（Ctrl+S） */
  onSave?: () => void;
  /** 暗色主题 */
  dark?: boolean;
  /** 占位符 */
  placeholder?: string;
  className?: string;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 代码编辑器组件，使用 CodeMirror 6 内核，支持语法高亮、行号、折叠、搜索、自动补全
 */
export function CodeEditor({
  value,
  language,
  readOnly = false,
  onChange,
  onSave,
  dark = true,
  placeholder,
  className,
}: CodeEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  // 用 Compartment 实现运行时切换 language/readOnly 而不重建 editor
  const languageCompartment = useRef(new Compartment());
  const readOnlyCompartment = useRef(new Compartment());
  const onChangeRef = useRef(onChange);
  const onSaveRef = useRef(onSave);

  // 保持回调引用最新，避免 editor 重建
  useEffect(() => {
    onChangeRef.current = onChange;
    onSaveRef.current = onSave;
  }, [onChange, onSave]);

  // 初始化编辑器（仅一次）
  useEffect(() => {
    if (!hostRef.current) return;

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged && onChangeRef.current) {
        onChangeRef.current(update.state.doc.toString());
      }
    });

    const saveKeymap = keymap.of([
      {
        key: "Mod-s",
        preventDefault: true,
        run: () => {
          onSaveRef.current?.();
          return true;
        },
      },
    ]);

    const state = EditorState.create({
      doc: value,
      extensions: [
        lineNumbers(),
        foldGutter(),
        history(),
        drawSelection(),
        dropCursor(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        indentUnit.of("  "),
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        bracketMatching(),
        closeBrackets(),
        autocompletion(),
        rectangularSelection(),
        crosshairCursor(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        highlightActiveLineGutter(),
        keymap.of([
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          ...completionKeymap,
          indentWithTab,
        ]),
        saveKeymap,
        updateListener,
        EditorView.lineWrapping,
        EditorState.readOnly.of(readOnly),
        languageCompartment.current.of([]),
        readOnlyCompartment.current.of(EditorState.readOnly.of(readOnly)),
        ...(dark ? [oneDark] : []),
        EditorView.theme({
          "&": {
            height: "100%",
            fontSize: "13px",
          },
          ".cm-scroller": {
            fontFamily: "var(--font-mono, monospace)",
          },
          ".cm-content": {
            padding: "8px 0",
          },
          ".cm-gutters": {
            border: "none",
            background: "transparent",
          },
        }),
      ],
    });

    const view = new EditorView({
      state,
      parent: hostRef.current,
    });
    viewRef.current = view;

    // 异步加载初始语言扩展并 reconfigure
    let cancelled = false;
    getLanguageExtension(language).then((ext) => {
      if (cancelled || !viewRef.current) return;
      viewRef.current.dispatch({
        effects: languageCompartment.current.reconfigure(ext),
      });
    });

    return () => {
      cancelled = true;
      view.destroy();
      viewRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 外部 value 变化时同步到编辑器
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    if (current !== value) {
      view.dispatch({
        changes: { from: 0, to: current.length, insert: value },
      });
    }
  }, [value]);

  // 语言切换
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    let cancelled = false;
    getLanguageExtension(language).then((ext) => {
      if (cancelled || !viewRef.current) return;
      viewRef.current.dispatch({
        effects: languageCompartment.current.reconfigure(ext),
      });
    });
    return () => {
      cancelled = true;
    };
  }, [language]);

  // 只读切换
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: readOnlyCompartment.current.reconfigure(EditorState.readOnly.of(readOnly)),
    });
  }, [readOnly]);

  return (
    <div
      ref={hostRef}
      className={cn("h-full w-full overflow-hidden", className)}
      data-placeholder={placeholder}
    />
  );
}