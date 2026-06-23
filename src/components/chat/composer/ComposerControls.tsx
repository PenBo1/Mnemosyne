import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { ArrowUpIcon, SquareIcon, PaperclipIcon } from "lucide-react";
import { useI18n } from "@/lib/i18n";

interface ComposerControlsProps {
  streaming?: boolean;
  disabled?: boolean;
  canSubmit: boolean;
  onSubmit: () => void;
  onCancel?: () => void;
  onAttach?: () => void;
  className?: string;
}

export function ComposerControls({
  streaming,
  disabled,
  canSubmit,
  onSubmit,
  onCancel,
  onAttach,
  className,
}: ComposerControlsProps) {
  const { t } = useI18n();

  return (
    <div className={cn("flex items-center gap-1", className)}>
      {/* Attachment button */}
      {onAttach && (
        <Button
          variant="ghost"
          size="icon-sm"
          disabled={disabled || streaming}
          onClick={onAttach}
          aria-label={t.agentChat.showMaterials}
          className="text-muted-foreground hover:text-foreground"
        >
          <PaperclipIcon className="size-4" />
        </Button>
      )}

      {/* Send/Stop button */}
      {streaming ? (
        <Button
          variant="outline"
          size="sm"
          disabled={disabled}
          onClick={onCancel}
          aria-label={t.agentChat.stop}
        >
          <SquareIcon className="size-3" />
          <span className="ml-1">{t.agentChat.stop}</span>
        </Button>
      ) : (
        <Button
          size="icon"
          disabled={disabled || !canSubmit}
          onClick={onSubmit}
          aria-label={t.agentChat.send}
          className="rounded-full bg-foreground text-background hover:bg-foreground/90 disabled:bg-foreground/30"
        >
          <ArrowUpIcon className="size-5" />
        </Button>
      )}
    </div>
  );
}