/**
 * ═══════════════════════════════════════════════════════════════════════════
 * AISettings - AI 与智能体设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { CpuIcon, BoxesIcon, MessageSquareIcon, BotIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { ModelSettings } from "./ModelSettings";
import { EmbeddingSettings } from "./EmbeddingSettings";
import { PromptsSettings } from "./PromptsSettings";
import { AgentsSettings } from "./AgentsSettings";

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * AI 与智能体设置页面，包含模型、嵌入、提示词、智能体四个子页面
 */
export function AISettings() {
  const { t } = useI18n();

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.aiProvider}</PageTitle>
          <PageDescription>{t.settings.aiProviderDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      <Tabs defaultValue="model" className="w-full">
        <TabsList className="mb-4">
          <TabsTrigger value="model" className="gap-1.5">
            <CpuIcon className="size-3.5" />
            {t.settings.model}
          </TabsTrigger>
          <TabsTrigger value="embedding" className="gap-1.5">
            <BoxesIcon className="size-3.5" />
            {t.settings.embedding}
          </TabsTrigger>
          <TabsTrigger value="prompts" className="gap-1.5">
            <MessageSquareIcon className="size-3.5" />
            {t.settings.prompts}
          </TabsTrigger>
          <TabsTrigger value="agents" className="gap-1.5">
            <BotIcon className="size-3.5" />
            {t.settings.agents}
          </TabsTrigger>
        </TabsList>

        {/* ── 模型设置 ──────────────────────────────────────────────────────── */}
        <TabsContent value="model">
          <div className="-mt-6">
            <ModelSettings />
          </div>
        </TabsContent>

        {/* ── 嵌入设置 ──────────────────────────────────────────────────────── */}
        <TabsContent value="embedding">
          <div className="-mt-6">
            <EmbeddingSettings />
          </div>
        </TabsContent>

        {/* ── 提示词设置 ────────────────────────────────────────────────────── */}
        <TabsContent value="prompts">
          <div className="-mt-6">
            <PromptsSettings />
          </div>
        </TabsContent>

        {/* ── 智能体设置 ────────────────────────────────────────────────────── */}
        <TabsContent value="agents">
          <div className="-mt-6">
            <AgentsSettings />
          </div>
        </TabsContent>
      </Tabs>
    </PageContainer>
  );
}