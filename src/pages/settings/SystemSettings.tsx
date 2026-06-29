import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { InfoIcon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";

export function SystemSettings() {
  const { t } = useI18n();

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.system}</PageTitle>
          <PageDescription>{t.settings.systemDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      {/* 系统信息 */}
      <Card className="py-0 gap-0">
        <CardContent className="flex items-center justify-between border-b py-3">
          <span className="text-sm font-medium">{t.common.app}</span>
          <Badge variant="secondary">Mnemosyne v0.1.0</Badge>
        </CardContent>
        <CardContent className="flex items-center justify-between border-b py-3">
          <span className="text-sm font-medium">{t.common.framework}</span>
          <span className="text-sm text-muted-foreground">Tauri v2 + React 19</span>
        </CardContent>
        <CardContent className="flex items-center justify-between py-3">
          <span className="text-sm font-medium">{t.common.runtime}</span>
          <span className="text-sm text-muted-foreground">Vite + TypeScript</span>
        </CardContent>
      </Card>

      {/* 即将推出 */}
      <Card>
        <CardContent className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <InfoIcon className="size-4 text-muted-foreground" />
            <span className="text-sm font-medium">{t.settings.systemDesc}</span>
          </div>
          <p className="text-sm text-muted-foreground">
            {t.ai.comingSoon}
          </p>
        </CardContent>
      </Card>
    </PageContainer>
  );
}
