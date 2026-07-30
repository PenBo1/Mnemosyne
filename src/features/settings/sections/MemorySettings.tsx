// 记忆与学习设置（合并版）—— 短期记忆、项目记忆、学习偏好、每日摘要、技能演化
// 合并自：ShortTermMemorySettings + ProjectMemorySettings + LearnedPreferencesSettings + DailySummarySettings + SkillEvolutionSettings

import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { BrainIcon, FolderCogIcon, HeartIcon, CalendarClockIcon, SparklesIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { ShortTermMemorySettings } from "./ShortTermMemorySettings";
import { ProjectMemorySettings } from "./ProjectMemorySettings";
import { LearnedPreferencesSettings } from "./LearnedPreferencesSettings";
import { DailySummarySettings } from "./DailySummarySettings";
import { SkillEvolutionSettings } from "./SkillEvolutionSettings";

export function MemorySettings() {
  const { t } = useI18n();
  const tm = t.settings.memorySettings;

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{tm?.title ?? "Memory & Learning"}</PageTitle>
          <PageDescription>{tm?.description ?? "Manage memory systems and learned preferences"}</PageDescription>
        </PageHeading>
      </PageHeader>

      <Tabs defaultValue="shortTerm" className="w-full">
        <TabsList className="mb-4 flex-wrap gap-1">
          <TabsTrigger value="shortTerm" className="gap-1.5">
            <BrainIcon className="size-3.5" />
            {tm?.shortTerm ?? t.settings.shortTermMemoryLabel}
          </TabsTrigger>
          <TabsTrigger value="project" className="gap-1.5">
            <FolderCogIcon className="size-3.5" />
            {tm?.project ?? t.settings.projectMemoryLabel}
          </TabsTrigger>
          <TabsTrigger value="learned" className="gap-1.5">
            <HeartIcon className="size-3.5" />
            {tm?.learned ?? t.settings.learnedPreferencesLabel}
          </TabsTrigger>
          <TabsTrigger value="daily" className="gap-1.5">
            <CalendarClockIcon className="size-3.5" />
            {tm?.daily ?? t.settings.dailySummaryLabel}
          </TabsTrigger>
          <TabsTrigger value="skill" className="gap-1.5">
            <SparklesIcon className="size-3.5" />
            {tm?.skill ?? t.settings.skillEvolutionLabel}
          </TabsTrigger>
        </TabsList>

        {/* 短期记忆 */}
        <TabsContent value="shortTerm">
          <div className="-mt-6">
            <ShortTermMemorySettings />
          </div>
        </TabsContent>

        {/* 项目记忆 */}
        <TabsContent value="project">
          <div className="-mt-6">
            <ProjectMemorySettings />
          </div>
        </TabsContent>

        {/* 学习偏好 */}
        <TabsContent value="learned">
          <div className="-mt-6">
            <LearnedPreferencesSettings />
          </div>
        </TabsContent>

        {/* 每日摘要 */}
        <TabsContent value="daily">
          <div className="-mt-6">
            <DailySummarySettings />
          </div>
        </TabsContent>

        {/* 技能演化 */}
        <TabsContent value="skill">
          <div className="-mt-6">
            <SkillEvolutionSettings />
          </div>
        </TabsContent>
      </Tabs>
    </PageContainer>
  );
}