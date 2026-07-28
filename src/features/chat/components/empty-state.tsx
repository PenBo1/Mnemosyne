/**
 * ═══════════════════════════════════════════════════════════════════════════
 * EmptyState - 聊天页面空状态欢迎页组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo } from "react";
import { BookOpen, Users, Globe, GitBranch, type LucideIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
  EmptyDescription,
  EmptyMedia,
} from "@/components/ui/empty";
import { Button } from "@/components/ui/button";

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 空状态欢迎页，展示功能引导卡片
 */
export const EmptyState = memo(function EmptyState() {
  const { t } = useI18n();

  const cards: Array<{ icon: LucideIcon; title: string; prompt: string }> = [
    { icon: BookOpen, title: t.agentChat.featureNovel, prompt: t.agentChat.featureNovelPrompt },
    { icon: Users, title: t.agentChat.featureCharacter, prompt: t.agentChat.featureCharacterPrompt },
    { icon: Globe, title: t.agentChat.featureWorldbuilding, prompt: t.agentChat.featureWorldbuildingPrompt },
    { icon: GitBranch, title: t.agentChat.featurePlotAnalysis, prompt: t.agentChat.featurePlotAnalysisPrompt },
  ];

  return (
    <Empty>
      <EmptyHeader>
        <EmptyTitle>{t.agentChat.welcomeTitle}</EmptyTitle>
        <EmptyDescription>{t.agentChat.welcomeHint}</EmptyDescription>
      </EmptyHeader>

      <div className="grid w-full max-w-lg grid-cols-2 gap-2.5">
        {cards.map((card) => (
          <Button
            key={card.title}
            variant="outline"
            className="group h-auto flex-col items-start gap-2 rounded-xl p-4 text-left"
            onClick={() => {
              window.dispatchEvent(new CustomEvent("chat:prompt", { detail: card.prompt }));
            }}
          >
            <EmptyMedia variant="icon">
              <card.icon />
            </EmptyMedia>
            <span className="text-xs font-medium text-foreground">{card.title}</span>
          </Button>
        ))}
      </div>
    </Empty>
  );
});