import { useRef, type ReactNode } from "react";
import { Check, Copy } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { useCopyFeedback } from "@/hooks/useCopyFeedback";

/**
 * Code block container: header with language label + copy button,
 * hljs-highlighted code below.
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
    <div className="my-3 overflow-hidden rounded-[var(--radius-6)] border border-[var(--border-neutral-l1)] bg-[var(--bg-base-tertiary)]">
      <div className="flex items-center justify-between border-b border-[var(--border-neutral-l1)] px-3 py-1.5">
        <span className="text-[11px] font-medium uppercase tracking-wide text-[var(--text-tertiary)]">
          {language ?? "text"}
        </span>
        <button
          type="button"
          onClick={handleCopy}
          className="flex items-center gap-1 rounded-[var(--radius-4)] px-1.5 py-0.5 text-[11px] text-[var(--text-tertiary)] transition-colors hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-secondary)]"
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
