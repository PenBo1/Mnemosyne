import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  AlertTriangleIcon,
  InfoIcon,
  CheckCircleIcon,
  ShieldIcon,
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
        <h2 className="text-lg font-semibold flex items-center gap-2">
          <ShieldIcon className="size-5" />
          {t.audit.securityLevel}
        </h2>
        <div className="mt-3 flex items-center gap-3">
          <Badge
            variant={
              status?.security_level === "Restricted"
                ? "default"
                : status?.security_level === "Strict"
                ? "secondary"
                : "outline"
            }
            className="text-lg px-4 py-1"
          >
            {status?.security_level || t.audit.unknown}
          </Badge>
          <span className="text-sm text-muted-foreground">
            {t.audit.policy}: {status?.policy_name || t.audit.none}
          </span>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <CheckCircleIcon className="text-green-500" />
              {t.audit.fileSystemRules}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <FieldGroup>
              {status?.fs_status.allowed_patterns.map((pattern, i) => (
                <Field key={`allow-${i}`} orientation="horizontal">
                  <Badge variant="outline" className="bg-green-500/10 text-green-600 shrink-0">
                    {t.audit.allowed}
                  </Badge>
                  <code className="text-sm">{pattern}</code>
                </Field>
              ))}
              {status?.fs_status.denied_patterns.map((pattern, i) => (
                <Field key={`deny-${i}`} orientation="horizontal">
                  <Badge variant="outline" className="bg-red-500/10 text-red-600 shrink-0">
                    {t.audit.denied}
                  </Badge>
                  <code className="text-sm">{pattern}</code>
                </Field>
              ))}
            </FieldGroup>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <InfoIcon className="text-blue-500" />
              {t.audit.networkRules}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <FieldGroup>
              {status?.net_status.allowed_hosts.map((host, i) => (
                <Field key={`allow-${i}`} orientation="horizontal">
                  <Badge variant="outline" className="bg-green-500/10 text-green-600 shrink-0">
                    {t.audit.allowed}
                  </Badge>
                  <code className="text-sm">{host}</code>
                </Field>
              ))}
              {status?.net_status.denied_hosts.map((host, i) => (
                <Field key={`deny-${i}`} orientation="horizontal">
                  <Badge variant="outline" className="bg-red-500/10 text-red-600 shrink-0">
                    {t.audit.denied}
                  </Badge>
                  <code className="text-sm">{host}</code>
                </Field>
              ))}
            </FieldGroup>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <AlertTriangleIcon className="text-yellow-500" />
              {t.audit.commandRules}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">
              {t.audit.commandRuleCount.replace("{count}", String(status?.exec_rule_count || 0))}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t.audit.resourceLimits}</CardTitle>
          </CardHeader>
          <CardContent>
            <FieldGroup>
              <Field orientation="horizontal">
                <FieldLabel className="flex-1 text-muted-foreground">{t.audit.execTimeout}</FieldLabel>
                <span className="text-sm font-medium">{status?.timeout_secs || 0} {t.audit.seconds}</span>
              </Field>
              <Field orientation="horizontal">
                <FieldLabel className="flex-1 text-muted-foreground">{t.audit.totalRules}</FieldLabel>
                <span className="text-sm font-medium">{status?.fs_status.rule_count || 0}</span>
              </Field>
            </FieldGroup>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
