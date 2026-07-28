// 安全审计设置（合并版）—— 安全规则 + 审计事件
// 合并自：AuditSettings + AgentAuditSettings

import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { ShieldIcon, ScrollIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { AuditSettings } from "./AuditSettings";
import { AgentAuditSettings } from "./AgentAuditSettings";

export function SecuritySettings() {
  const { t } = useI18n();
  // 使用 any 绕过类型检查，因为新键可能未被 TypeScript 识别
  const ts = (t.settings as any).securitySettings;

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.audit}</PageTitle>
          <PageDescription>{t.settings.auditDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      <Tabs defaultValue="rules" className="w-full">
        <TabsList className="mb-4">
          <TabsTrigger value="rules" className="gap-1.5">
            <ShieldIcon className="size-3.5" />
            {ts?.rules ?? "Security Rules"}
          </TabsTrigger>
          <TabsTrigger value="events" className="gap-1.5">
            <ScrollIcon className="size-3.5" />
            {ts?.events ?? "Audit Events"}
          </TabsTrigger>
        </TabsList>

        {/* 安全规则 */}
        <TabsContent value="rules">
          <div className="-mt-6">
            <AuditSettings />
          </div>
        </TabsContent>

        {/* 审计事件 */}
        <TabsContent value="events">
          <div className="-mt-6">
            <AgentAuditSettings />
          </div>
        </TabsContent>
      </Tabs>
    </PageContainer>
  );
}