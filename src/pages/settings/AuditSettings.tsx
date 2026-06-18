import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import {
  AlertTriangleIcon,
  InfoIcon,
  CheckCircleIcon,
} from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { useSandboxStatus } from "@/hooks/useSandbox";

export function AuditSettings() {
  const { t } = useI18n();
  const { status, loading } = useSandboxStatus();

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Spinner className="size-6" />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">{t.audit.securityLevel}</h1>
        <div className="mt-2 flex items-center gap-3">
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
      </div>

      {/* File System Rules */}
      <div className="rounded-lg border bg-card">
        <div className="px-4 py-3 border-b">
          <div className="flex items-center gap-2">
            <CheckCircleIcon className="size-4 text-green-500" />
            <span className="text-sm font-medium">{t.audit.fileSystemRules}</span>
          </div>
        </div>
        {status?.fs_status.allowed_patterns.map((pattern, i) => (
          <div key={`allow-${i}`} className="flex items-center justify-between px-4 py-3 border-b last:border-b-0">
            <span className="text-xs text-muted-foreground">{pattern}</span>
            <Badge variant="outline" className="bg-green-500/10 text-green-600 text-xs">
              {t.audit.allowed}
            </Badge>
          </div>
        ))}
        {status?.fs_status.denied_patterns.map((pattern, i) => (
          <div key={`deny-${i}`} className="flex items-center justify-between px-4 py-3 border-b last:border-b-0">
            <span className="text-xs text-muted-foreground">{pattern}</span>
            <Badge variant="outline" className="bg-red-500/10 text-red-600 text-xs">
              {t.audit.denied}
            </Badge>
          </div>
        ))}
      </div>

      {/* Network Rules */}
      <div className="rounded-lg border bg-card">
        <div className="px-4 py-3 border-b">
          <div className="flex items-center gap-2">
            <InfoIcon className="size-4 text-blue-500" />
            <span className="text-sm font-medium">{t.audit.networkRules}</span>
          </div>
        </div>
        {status?.net_status.allowed_hosts.map((host, i) => (
          <div key={`allow-${i}`} className="flex items-center justify-between px-4 py-3 border-b last:border-b-0">
            <span className="text-xs text-muted-foreground">{host}</span>
            <Badge variant="outline" className="bg-green-500/10 text-green-600 text-xs">
              {t.audit.allowed}
            </Badge>
          </div>
        ))}
        {status?.net_status.denied_hosts.map((host, i) => (
          <div key={`deny-${i}`} className="flex items-center justify-between px-4 py-3 border-b last:border-b-0">
            <span className="text-xs text-muted-foreground">{host}</span>
            <Badge variant="outline" className="bg-red-500/10 text-red-600 text-xs">
              {t.audit.denied}
            </Badge>
          </div>
        ))}
      </div>

      {/* Command Rules */}
      <div className="rounded-lg border bg-card">
        <div className="px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <AlertTriangleIcon className="size-4 text-yellow-500" />
              <span className="text-sm font-medium">{t.audit.commandRules}</span>
            </div>
            <span className="text-xs text-muted-foreground">
              {t.audit.commandRuleCount.replace("{count}", String(status?.exec_rule_count || 0))}
            </span>
          </div>
        </div>
      </div>

      {/* Resource Limits */}
      <div className="rounded-lg border bg-card">
        <div className="px-4 py-3 border-b">
          <span className="text-sm font-medium">{t.audit.resourceLimits}</span>
        </div>
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <span className="text-xs text-muted-foreground">{t.audit.execTimeout}</span>
          <span className="text-xs font-medium">{status?.timeout_secs || 0} {t.audit.seconds}</span>
        </div>
        <div className="flex items-center justify-between px-4 py-3">
          <span className="text-xs text-muted-foreground">{t.audit.totalRules}</span>
          <span className="text-xs font-medium">{status?.fs_status.rule_count || 0}</span>
        </div>
      </div>
    </div>
  );
}
