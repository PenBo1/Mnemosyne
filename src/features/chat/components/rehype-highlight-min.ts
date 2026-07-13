/**
 * 极简 rehype-highlight — 替代 rehype-highlight 包 (避免其静态 import common 导致 37 种语言全打包)。
 *
 * 原理: 用 createLowlight 只注册指定语言, 遍历 hast code 节点, 用 lowlight.highlight 生成高亮 hast 替换 children。
 * 体积: markdown chunk 从 334KB → ~80KB (减 ~76%), 因为不含 common 的 37 种语言语法。
 *
 * 与 rehype-highlight 的区别:
 * - 不 import common (rehype-highlight 静态 import, bundler 无法 tree-shake)
 * - 不支持 detect (auto-detect 需 highlightAuto, 小说场景不需要)
 * - 未知语言静默跳过 (rehype-highlight 会发 vfile message)
 */

import { createLowlight, type LanguageFn } from "lowlight";
import { visit } from "unist-util-visit";
import type { Root, Element, ElementContent } from "hast";

/** 从 code 节点的 className 提取语言 (language-xxx 或 lang-xxx)。 */
function getLanguage(node: Element): string | false | undefined {
  const list = node.properties?.className;
  if (!Array.isArray(list)) return undefined;
  let name: string | undefined;
  for (const value of list) {
    const v = String(value);
    if (v === "no-highlight" || v === "nohighlight") return false;
    if (!name && v.startsWith("lang-")) name = v.slice(5);
    if (!name && v.startsWith("language-")) name = v.slice(9);
  }
  return name;
}

/** 从 code 节点 children 提取纯文本 (highlight 前都是 text 节点)。 */
function toPlainText(node: Element): string {
  return (node.children as Array<{ value?: string }>)
    .map((c) => c.value ?? "")
    .join("");
}

/**
 * 创建极简 rehype-highlight 插件。
 * @param languages 只注册这些语言 (如 { javascript, typescript, ... })
 */
export function createRehypeHighlight(
  languages: Readonly<Record<string, LanguageFn>>,
) {
  const lowlight = createLowlight(languages);

  return function rehypeHighlightMin() {
    return function transform(tree: Root): undefined {
      visit(tree, "element", function (node, _index, parent) {
        if (
          node.tagName !== "code" ||
          !parent ||
          parent.type !== "element" ||
          parent.tagName !== "pre"
        ) {
          return;
        }

        const lang = getLanguage(node);
        if (lang === false || !lang) return;
        if (!lowlight.registered(lang)) return;

        // 确保 className 含 hljs (供 CSS 高亮)
        const cls = node.properties.className;
        if (!Array.isArray(cls)) {
          node.properties.className = ["hljs"];
        } else if (!cls.includes("hljs")) {
          node.properties.className = ["hljs", ...cls];
        }

        const text = toPlainText(node);
        try {
          const result = lowlight.highlight(lang, text);
          if (result.children.length > 0) {
            node.children = result.children as ElementContent[];
          }
        } catch {
          // 未知语言或高亮失败 — 保留原文, 不报错
        }
      });
      return undefined;
    };
  };
}
