import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { CodeBlock } from "./CodeBlock";

const components: Components = {
  pre: (props) => <>{props.children}</>,
  code: (props) => {
    const className = props.className ?? "";
    // rehype-highlight adds hljs/language-xxx class to block code; inline code has none
    const isBlock =
      className.includes("hljs") || className.includes("language-");
    if (!isBlock) {
      return (
        <code className="rounded bg-[var(--bg-overlay-l2)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--code-text)]">
          {props.children}
        </code>
      );
    }
    const match = /language-(\w+)/.exec(className);
    return (
      <CodeBlock language={match?.[1] ?? null} className={className}>
        {props.children}
      </CodeBlock>
    );
  },
  a: (props) => (
    <a
      {...props}
      target="_blank"
      rel="noreferrer"
      className="text-[var(--status-primary-default)] underline underline-offset-2 hover:opacity-80"
    />
  ),
  table: (props) => (
    <div className="my-3 overflow-x-auto">
      <table {...props} className="w-full border-collapse text-sm" />
    </div>
  ),
  th: (props) => (
    <th
      {...props}
      className="border border-[var(--border-neutral-l1)] bg-[var(--bg-overlay-l1)] px-3 py-1.5 text-left font-medium"
    />
  ),
  td: (props) => <td {...props} className="border border-[var(--border-neutral-l1)] px-3 py-1.5" />,
  ul: (props) => <ul {...props} className="my-2 list-disc flex flex-col gap-1 pl-6" />,
  ol: (props) => <ol {...props} className="my-2 list-decimal flex flex-col gap-1 pl-6" />,
  li: (props) => <li {...props} className="leading-relaxed" />,
  p: (props) => (
    <p {...props} className="my-2 leading-[1.7] first:mt-0 last:mb-0" />
  ),
  h1: (props) => (
    <h1 {...props} className="mb-2 mt-4 text-lg font-semibold first:mt-0" />
  ),
  h2: (props) => (
    <h2 {...props} className="mb-2 mt-4 text-base font-semibold first:mt-0" />
  ),
  h3: (props) => (
    <h3 {...props} className="mb-1.5 mt-3 text-sm font-semibold first:mt-0" />
  ),
  h4: (props) => (
    <h4 {...props} className="mb-1 mt-2 text-sm font-medium first:mt-0" />
  ),
  hr: () => <hr className="my-4 border-[var(--border-neutral-l1)]" />,
  blockquote: (props) => (
    <blockquote
      {...props}
      className="my-3 border-l-2 border-[var(--border-neutral-l2)] pl-3 text-[var(--text-secondary)]"
    />
  ),
};

export function MarkdownRenderer({ content }: { content: string }) {
  return (
    <div className="text-sm text-[var(--text-default)]">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[[rehypeHighlight, { detect: true, ignoreMissing: true }]]}
        components={components}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
