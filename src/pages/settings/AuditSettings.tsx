import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import {
  AlertTriangleIcon,
  InfoIcon,
  CheckCircleIcon,
} from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { useSandboxStatus } from "@/features/sandbox/hooks/useSandbox";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
} from "@/components/shared/page-layout";
import { LoadingState } from "@/components/shared/state";

export function AuditSettings() {
  const { t } = useI18n();
  const { status, loading } = useSandboxStatus();

  if (loading) {
    return (
      <PageContainer scrollable={false}>
        <LoadingState label={t.common.loading} />
      </PageContainer>
    );
  }

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.audit.securityLevel}</PageTitle>
          <div className="flex items-center gap-3">
            <Badge
              variant={
                status?.security_level === "Restricted"
                  ? "default"
                  : status?.security_level === "Strict"
                  ? "secondary"
                  : "outline"
              }
            >
              {status?.security_level || t.audit.unknown}
            </Badge>
            <span className="text-sm text-muted-foreground">
              {t.audit.policy}: {status?.policy_name || t.audit.none}
            </span>
          </div>
        </PageHeading>
      </PageHeader>

      {/* 文件系统规则 */}
      <Card className="py-0 gap-0">
        <CardContent className="flex items-center gap-2 border-b py-3">
          <CheckCircleIcon className="size-4 text-emerald-600 dark:text-emerald-400" />
          <span className="text-sm font-medium">{t.audit.fileSystemRules}</span>
        </CardContent>
        {status?.fs_status.allowed_patterns.map((pattern, i) => (
          <div key={`allow-${i}`} className="flex items-center justify-between border-b px-4 py-3 last:border-b-0">
            <span className="text-xs text-muted-foreground">{pattern}</span>
            <Badge variant="outline" className="bg-primary/10 text-primary text-xs">
              {t.audit.allowed}
            </Badge>
          </div>
        ))}
        {status?.fs_status.denied_patterns.map((pattern, i) => (
          <div key={`deny-${i}`} className="flex items-center justify-between border-b px-4 py-3 last:border-b-0">
            <span className="text-xs text-muted-foreground">{pattern}</span>
            <Badge variant="outline" className="bg-destructive/10 text-destructive text-xs">
              {t.audit.denied}
            </Badge>
          </div>
        ))}
      </Card>

      {/* 网络规则 */}
      <Card className="py-0 gap-0">
        <CardContent className="flex items-center gap-2 border-b py-3">
          <InfoIcon className="size-4 text-primary" />
          <span className="text-sm font-medium">{t.audit.networkRules}</span>
        </CardContent>
        {status?.net_status.allowed_hosts.map((host, i) => (
          <div key={`allow-${i}`} className="flex items-center justify-between border-b px-4 py-3 last:border-b-0">
            <span className="text-xs text-muted-foreground">{host}</span>
            <Badge variant="outline" className="bg-primary/10 text-primary text-xs">
              {t.audit.allowed}
            </Badge>
          </div>
        ))}
        {status?.net_status.denied_hosts.map((host, i) => (
          <div key={`deny-${i}`} className="flex items-center justify-between border-b px-4 py-3 last:border-b-0">
            <span className="text-xs text-muted-foreground">{host}</span>
            <Badge variant="outline" className="bg-destructive/10 text-destructive text-xs">
              {t.audit.denied}
            </Badge>
          </div>
        ))}
      </Card>

      {/* 命令规则 */}
      <Card>
        <CardContent className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <AlertTriangleIcon className="size-4 text-muted-foreground" />
            <span className="text-sm font-medium">{t.audit.commandRules}</span>
          </div>
          <span className="text-xs text-muted-foreground">
            {t.audit.commandRuleCount.replace("{count}", String(status?.exec_rule_count || 0))}
          </span>
        </CardContent>
      </Card>

      {/* 资源限制 */}
      <Card className="py-0 gap-0">
        <CardContent className="border-b py-3">
          <span className="text-sm font-medium">{t.audit.resourceLimits}</span>
        </CardContent>
        <CardContent className="flex items-center justify-between border-b py-3">
          <span className="text-xs text-muted-foreground">{t.audit.execTimeout}</span>
          <span className="text-xs font-medium">{status?.timeout_secs || 0} {t.audit.seconds}</span>
        </CardContent>
        <CardContent className="flex items-center justify-between py-3">
          <span className="text-xs text-muted-foreground">{t.audit.totalRules}</span>
          <span className="text-xs font-medium">{status?.fs_status.rule_count || 0}</span>
        </CardContent>
      </Card>
    </PageContainer>
  );
}
