import { BotIcon } from "lucide-react";
import { useI18n } from "@/shared/i18n";

export function EmptyState() {
  const { t } = useI18n();
  return (
    <div className="flex h-full flex-col items-center justify-center gap-5 text-center">
      <div className="flex size-14 items-center justify-center rounded-[var(--radius-8)] border border-border bg-card shadow-sm">
        <BotIcon className="size-6 text-primary" />
      </div>
      <div className="flex flex-col gap-1.5">
        <h2 className="text-base font-semibold text-foreground">
          {t.agentChat.welcomeTitle}
        </h2>
        <p className="max-w-sm text-sm leading-relaxed text-muted-foreground">
          {t.agentChat.welcomeHint}
        </p>
      </div>
    </div>
  );
}
