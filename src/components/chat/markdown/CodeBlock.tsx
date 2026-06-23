import { useState, useCallback } from "react";
import { cn } from "@/lib/utils";
import { CheckIcon, CopyIcon } from "lucide-react";
import { Button } from "@/components/ui/button";

interface CodeBlockProps {
  inline?: boolean;
  className?: string;
  children: React.ReactNode;
}

export function CodeBlock({ inline, className, children }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  const language = className?.replace("language-", "") || "";

  const handleCopy = useCallback(() => {
    const text = String(children).replace(/\n$/, "");
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [children]);

  if (inline) {
    return (
      <code className="bg-muted px-1.5 py-0.5 rounded text-sm font-mono">
        {children}
      </code>
    );
  }

  return (
    <div className="relative group my-2">
      <code className={cn("block font-mono text-sm", className)}>
        {children}
      </code>
      <div className="absolute top-2 right-2 flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={handleCopy}
          aria-label="Copy code"
        >
          {copied ? (
            <CheckIcon className="size-3 text-green-500" />
          ) : (
            <CopyIcon className="size-3" />
          )}
        </Button>
        {language && (
          <span className="text-xs text-muted-foreground px-1">{language}</span>
        )}
      </div>
    </div>
  );
}