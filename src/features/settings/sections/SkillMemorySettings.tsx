/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SkillMemorySettings - 技能记忆设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { SkillEvolutionSettings } from "./SkillEvolutionSettings";

export function SkillMemorySettings() {
  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>Skill Memory</PageTitle>
          <PageDescription>Manage skill evolution and memory settings</PageDescription>
        </PageHeading>
      </PageHeader>

      <div className="-mt-6">
        <SkillEvolutionSettings />
      </div>
    </PageContainer>
  );
}
