import { useRef, type ReactNode } from "react";
import { Check, Copy } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { useCopyFeedback } from "@/hooks/useCopyFeedback";

/**
 * 代码块容器：顶部语言标签 + 复制按钮，下方 hljs 高亮代码。
 *
 * children 由 react-markdown + rehype-highlight 解析后的 hljs span 结构直接渲染，
 * 不重新解析，保留语法高亮。
 */
export function CodeBlock({
  language,
  className,
  children,
}: {
  language: string | null;
  className?: string;
  children: ReactNode;
}) {
  const codeRef = useRef<HTMLElement>(null);
  const { t } = useI18n();
  const { copied, copy } = useCopyFeedback();

  const handleCopy = () => {
    const text = codeRef.current?.textContent ?? "";
    copy(text);
  };

  return (
    <div className="my-3 overflow-hidden rounded-lg border border-[var(--border-neutral-l1)] bg-[var(--bg-base-tertiary)]">
      <div className="flex items-center justify-between border-b border-[var(--border-neutral-l1)] px-3 py-1.5">
        <span className="text-[11px] font-medium uppercase tracking-wide text-zinc-400">
          {language ?? "text"}
        </span>
        <button
          type="button"
          onClick={handleCopy}
          className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] text-zinc-400 transition-colors hover:bg-white/5 hover:text-zinc-200"
        >
          {copied ? <Check className="size-3" /> : <Copy className="size-3" />}
          <span>{copied ? t.agentChat.copied : t.agentChat.copyCode}</span>
        </button>
      </div>
      <pre className="overflow-x-auto p-3 text-[13px] leading-[1.5]">
        <code ref={codeRef} className={className}>
          {children}
        </code>
      </pre>
    </div>
  );
}
