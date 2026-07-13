import { memo } from "react";
import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import type { LanguageFn } from "lowlight";
import javascript from "highlight.js/lib/languages/javascript";
import typescript from "highlight.js/lib/languages/typescript";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import json from "highlight.js/lib/languages/json";
import bash from "highlight.js/lib/languages/bash";
import markdownLang from "highlight.js/lib/languages/markdown";
import { Separator } from "@/components/ui/separator";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Copy, Check } from "lucide-react";
import { useCopy } from "@/features/chat/hooks/use-copy";
import { createRehypeHighlight } from "@/features/chat/components/rehype-highlight-min";

const LANGUAGES: Readonly<Record<string, LanguageFn>> = {
  javascript,
  typescript,
  python,
  rust,
  json,
  bash,
  markdown: markdownLang,
};

const rehypeHighlight = createRehypeHighlight(LANGUAGES);

function CodeBlock({
  language,
  className,
  children,
}: {
  language: string | null;
  className: string;
  children: React.ReactNode;
}) {
  const { copied, copy } = useCopy();
  const codeText = String(children).replace(/\n$/, "");

  return (
    <div className="relative my-3 rounded-lg border border-border bg-muted/30">
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-border">
        <span className="text-xs font-medium text-muted-foreground">
          {language ?? "code"}
        </span>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={() => copy(codeText)}
              aria-label="Copy code"
            >
              {copied ? (
                <Check className="size-3" />
              ) : (
                <Copy className="size-3" />
              )}
            </Button>
          </TooltipTrigger>
          <TooltipContent>Copy code</TooltipContent>
        </Tooltip>
      </div>
      <pre className="overflow-x-auto p-3 text-xs">
        <code className={className}>{children}</code>
      </pre>
    </div>
  );
}

const components: Components = {
  pre: (props) => <>{props.children}</>,
  code: (props) => {
    const className = props.className ?? "";
    const isBlock =
      className.includes("hljs") || className.includes("language-");
    if (!isBlock) {
      return (
        <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[0.85em]">
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
      className="text-primary underline underline-offset-2 hover:opacity-80"
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
      className="border border-border bg-muted px-3 py-1.5 text-left font-medium"
    />
  ),
  td: (props) => (
    <td {...props} className="border border-border px-3 py-1.5" />
  ),
  ul: (props) => (
    <ul {...props} className="my-2 list-disc flex flex-col gap-1 pl-6" />
  ),
  ol: (props) => (
    <ol {...props} className="my-2 list-decimal flex flex-col gap-1 pl-6" />
  ),
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
  hr: () => <Separator className="my-4" />,
  blockquote: (props) => (
    <blockquote
      {...props}
      className="my-3 border-l-2 border-border pl-3 text-muted-foreground"
    />
  ),
};

export const MarkdownRenderer = memo(function MarkdownRenderer({
  content,
}: {
  content: string;
}) {
  return (
    <div className="text-sm text-foreground">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
        components={components}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
});