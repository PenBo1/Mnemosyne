/**
 * ═══════════════════════════════════════════════════════════════════════════
 * AuditEventsSettings - 审计事件设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { AgentAuditSettings } from "./AgentAuditSettings";

export function AuditEventsSettings() {
  const { t } = useI18n();

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.securitySettings.events}</PageTitle>
          <PageDescription>View audit events and agent actions</PageDescription>
        </PageHeading>
      </PageHeader>

      <div className="-mt-6">
        <AgentAuditSettings />
      </div>
    </PageContainer>
  );
}
