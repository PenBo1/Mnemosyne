import { BookOpen, Users, Globe, GitBranch, Sparkles } from "lucide-react";
import { useI18n } from "@/shared/i18n";

interface FeaturePill {
  icon: typeof BookOpen;
  labelKey: "featureNovel" | "featureCharacter" | "featureWorldbuilding" | "featurePlotAnalysis";
}

const FEATURES: FeaturePill[] = [
  { icon: BookOpen, labelKey: "featureNovel" },
  { icon: Users, labelKey: "featureCharacter" },
  { icon: Globe, labelKey: "featureWorldbuilding" },
  { icon: GitBranch, labelKey: "featurePlotAnalysis" },
];

export function EmptyState() {
  const { t } = useI18n();

  return (
    <div className="flex h-full flex-col items-center justify-center px-6">
      <div className="flex w-full max-w-xl flex-col items-center gap-6">
        {/* Logo / Icon */}
        <div className="flex size-16 items-center justify-center rounded-[var(--radius-8)] border border-[var(--border-neutral-l1)] bg-[var(--bg-base-secondary)]">
          <Sparkles className="size-7 text-[var(--status-primary-default)]" />
        </div>

        {/* Title */}
        <h1 className="text-xl font-semibold text-[var(--text-default)]">
          {t.agentChat.welcomeTitle}
        </h1>

        {/* Description */}
        <p className="max-w-md text-center text-sm leading-relaxed text-[var(--text-secondary)]">
          {t.agentChat.welcomeHint}
        </p>

        {/* Feature Pills */}
        <div className="flex flex-wrap items-center justify-center gap-2">
          {FEATURES.map((feature) => (
            <button
              key={feature.labelKey}
              type="button"
              className="flex items-center gap-1.5 rounded-[var(--radius-full)] border border-[var(--border-neutral-l1)] bg-[var(--bg-base-secondary)] px-3.5 py-1.5 text-xs font-medium text-[var(--text-secondary)] transition-colors hover:border-[var(--status-primary-default)]/30 hover:text-[var(--text-default)] hover:bg-[var(--status-primary-surface-l1)]"
            >
              <feature.icon className="size-3.5" />
              {t.agentChat[feature.labelKey]}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
