import { cn } from "@/lib/utils";
import { useI18n } from "@/lib/i18n";
import { SparklesIcon } from "lucide-react";

interface WelcomeIntroProps {
  className?: string;
}

export function WelcomeIntro({ className }: WelcomeIntroProps) {
  const { t } = useI18n();

  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center text-center px-4",
        className
      )}
    >
      <SparklesIcon className="size-8 text-primary mb-4 opacity-80" />
      <h1 className="text-xl font-semibold mb-2">{t.agentChat.welcomeTitle}</h1>
      <p className="text-sm text-muted-foreground max-w-md">
        {t.agentChat.welcomeHint}
      </p>
    </div>
  );
}