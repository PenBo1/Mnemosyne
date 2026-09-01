/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SecurityRulesSettings - 安全规则设置页面
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
import { AuditSettings } from "./AuditSettings";

export function SecurityRulesSettings() {
  const { t } = useI18n();

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.securitySettings.rules}</PageTitle>
          <PageDescription>Manage security rules and sandbox policies</PageDescription>
        </PageHeading>
      </PageHeader>

      <div className="-mt-6">
        <AuditSettings />
      </div>
    </PageContainer>
  );
}
