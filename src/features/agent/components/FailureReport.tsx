/**
 * ═══════════════════════════════════════════════════════════════════════════
 * FailureReport - Agent 执行失败报告组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo, useMemo } from "react";
import { 
  AlertTriangle, AlertCircle, Info, Wrench, 
  Zap, Brain, ShieldAlert, Link2, Settings
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { cn } from "@/lib/utils";
import { Card, CardHeader, CardTitle, CardContent, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

// ── 类型定义 ────────────────────────────────────────────────────────────────

type FailureType = 
  | "hallucinated_tool"
  | "scope_creep"
  | "context_loss"
  | "constraint_violation"
  | "cascading_error"
  | "tool_misuse_missing_params"
  | "tool_misuse_type_error"
  | "tool_misuse_critical_path"
  | "tool_misuse_dangerous_command";

type Severity = "error" | "warning" | "info";

export interface FailurePattern {
  type: FailureType;
  severity: Severity;
  message: string;
  suggestion: string;
  details?: Record<string, unknown>;
}

interface FailureReportProps {
  failures: FailurePattern[];
  onDismiss?: () => void;
  onAction?: (failure: FailurePattern) => void;
}

interface FailureCardProps {
  failure: FailurePattern;
  onAction?: (failure: FailurePattern) => void;
}

// ── 常量配置 ────────────────────────────────────────────────────────────────

const FAILURE_ICONS: Record<FailureType, React.ComponentType<{ className?: string }>> = {
  hallucinated_tool: Zap,
  scope_creep: AlertTriangle,
  context_loss: Brain,
  constraint_violation: ShieldAlert,
  cascading_error: Link2,
  tool_misuse_missing_params: Settings,
  tool_misuse_type_error: Settings,
  tool_misuse_critical_path: AlertTriangle,
  tool_misuse_dangerous_command: AlertCircle,
};

const SEVERITY_CONFIG: Record<Severity, { 
  icon: React.ComponentType<{ className?: string }>; 
  variant: "destructive" | "warning" | "info";
  className: string;
}> = {
  error: {
    icon: AlertCircle,
    variant: "destructive",
    className: "border-destructive/50 bg-destructive/5",
  },
  warning: {
    icon: AlertTriangle,
    variant: "warning",
    className: "border-warning/50 bg-warning/5",
  },
  info: {
    icon: Info,
    variant: "info",
    className: "border-info/50 bg-info/5",
  },
};

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 获取失败类型的标签文本
 */
function getFailureLabel(type: FailureType, t: Record<string, unknown>): string {
  const labels: Record<FailureType, string> = {
    hallucinated_tool: (t.failureReport as Record<string, string>)?.hallucinatedTool || "Hallucinated Tool",
    scope_creep: (t.failureReport as Record<string, string>)?.scopeCreep || "Scope Creep",
    context_loss: (t.failureReport as Record<string, string>)?.contextLoss || "Context Loss",
    constraint_violation: (t.failureReport as Record<string, string>)?.constraintViolation || "Constraint Violation",
    cascading_error: (t.failureReport as Record<string, string>)?.cascadingError || "Cascading Error",
    tool_misuse_missing_params: (t.failureReport as Record<string, string>)?.missingParams || "Missing Parameters",
    tool_misuse_type_error: (t.failureReport as Record<string, string>)?.typeError || "Type Error",
    tool_misuse_critical_path: (t.failureReport as Record<string, string>)?.criticalPath || "Critical Path",
    tool_misuse_dangerous_command: (t.failureReport as Record<string, string>)?.dangerousCommand || "Dangerous Command",
  };
  return labels[type];
}

// ── 子组件 ──────────────────────────────────────────────────────────────────

/**
 * 单个失败项卡片组件
 */
const FailureCard = memo(function FailureCard({ failure, onAction }: FailureCardProps) {
  const { t } = useI18n();
  const Icon = FAILURE_ICONS[failure.type];
  const config = SEVERITY_CONFIG[failure.severity];
  
  return (
    <div
      className={cn(
        "rounded-lg border p-3",
        config.className
      )}
    >
      <div className="flex items-start gap-2">
        <Icon className="size-4 shrink-0 mt-0.5" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <Badge variant={config.variant} className="text-[10px]">
              {getFailureLabel(failure.type, t)}
            </Badge>
            <Badge variant="outline" className="text-[10px]">
              {failure.severity}
            </Badge>
          </div>
          
          <p className="text-xs text-foreground mb-2">
            {failure.message}
          </p>
          
          <div className="rounded bg-muted/50 p-2 mb-2">
            <p className="text-[11px] text-muted-foreground flex items-start gap-1.5">
              <Wrench className="size-3 shrink-0 mt-0.5" />
              <span>{failure.suggestion}</span>
            </p>
          </div>
          
          {onAction && (
            <Button
              size="xs"
              variant="outline"
              onClick={() => onAction(failure)}
              className="mt-1"
            >
              {(t.failureReport as Record<string, string>)?.takeAction || "Take Action"}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
});

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 失败报告组件，用于展示 Agent 执行过程中的失败信息
 */
const FailureReport = memo(function FailureReport({ failures, onDismiss, onAction }: FailureReportProps) {
  const { t } = useI18n();

  const groupedFailures = useMemo(() => {
    const groups: Record<Severity, FailurePattern[]> = {
      error: [],
      warning: [],
      info: [],
    };
    for (const failure of failures) {
      groups[failure.severity].push(failure);
    }
    return groups;
  }, [failures]);

  if (failures.length === 0) return null;

  return (
    <Card className="max-w-xl mx-auto border-destructive/30 bg-destructive/5">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-1.5 text-destructive">
            <AlertTriangle className="size-4" />
            {(t.failureReport as Record<string, string>)?.title || "Failure Detected"}
          </CardTitle>
          {onDismiss && (
            <Button
              size="xs"
              variant="ghost"
              onClick={onDismiss}
            >
              {t.common.close}
            </Button>
          )}
        </div>
        <CardDescription>
          {(t.failureReport as Record<string, string>)?.description || "The agent encountered issues during execution"}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex flex-col gap-3">
          {groupedFailures.error.map((f) => (
            <FailureCard key={`${f.type}-${f.message}`} failure={f} onAction={onAction} />
          ))}
          {groupedFailures.warning.map((f) => (
            <FailureCard key={`${f.type}-${f.message}`} failure={f} onAction={onAction} />
          ))}
          {groupedFailures.info.map((f) => (
            <FailureCard key={`${f.type}-${f.message}`} failure={f} onAction={onAction} />
          ))}
        </div>
      </CardContent>
    </Card>
  );
});

export { FailureReport };