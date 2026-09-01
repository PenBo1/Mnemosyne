/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Agent 评估显示组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import React from "react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  AlertCircle,
  CheckCircle2,
  Clock,
  Shield,
  Target,
  Wrench,
  BookOpen,
} from "lucide-react";
import { useI18n, type Translations } from "@/locales/i18n";
import type {
  DimensionId,
  DimensionScore,
  Finding,
  EvidenceState,
  FindingSeverity,
  RepairProgress,
} from "./types";
import { DIMENSIONS } from "./types";

// ── 评分颜色工具 ────────────────────────────────────────────────────────────

/**
 * 根据评分获取颜色类名
 */
function getScoreColorClass(score: number): string {
  if (score >= 90) return "text-green-500";
  if (score >= 75) return "text-blue-500";
  if (score >= 60) return "text-yellow-500";
  return "text-red-500";
}

/**
 * 根据评分获取背景颜色类名
 */
function getScoreBgClass(score: number): string {
  if (score >= 90) return "bg-green-500";
  if (score >= 75) return "bg-blue-500";
  if (score >= 60) return "bg-yellow-500";
  return "bg-red-500";
}

/**
 * 根据严重程度获取图标和颜色
 */
function getSeverityStyle(severity: FindingSeverity): {
  icon: React.ReactNode;
  className: string;
} {
  switch (severity) {
    case "critical":
      return {
        icon: <AlertCircle className="h-4 w-4" />,
        className: "text-red-500 bg-red-50",
      };
    case "major":
      return {
        icon: <AlertCircle className="h-4 w-4" />,
        className: "text-orange-500 bg-orange-50",
      };
    case "minor":
      return {
        icon: <Clock className="h-4 w-4" />,
        className: "text-yellow-500 bg-yellow-50",
      };
    default:
      return {
        icon: <CheckCircle2 className="h-4 w-4" />,
        className: "text-gray-500 bg-gray-50",
      };
  }
}

/**
 * 获取修复进度样式
 */
function getRepairProgressStyle(progress: RepairProgress): {
  variant: "default" | "secondary" | "destructive" | "outline";
  className: string;
} {
  switch (progress) {
    case "verified":
      return { variant: "default", className: "bg-green-500" };
    case "partial":
      return { variant: "secondary", className: "bg-yellow-500" };
    case "blocked":
      return { variant: "destructive", className: "" };
    default:
      return { variant: "outline", className: "" };
  }
}

/**
 * 获取证据状态显示文本
 */
function getEvidenceStateLabel(state: EvidenceState, t: Translations): string {
  const labels: Record<EvidenceState, string> = {
    present: (t as unknown as { evaluationEvidencePresent?: string }).evaluationEvidencePresent || "存在",
    wired: (t as unknown as { evaluationEvidenceWired?: string }).evaluationEvidenceWired || "已连接",
    exercised: (t as unknown as { evaluationEvidenceExercised?: string }).evaluationEvidenceExercised || "已执行",
    outcome_supported: (t as unknown as { evaluationEvidenceOutcome?: string }).evaluationEvidenceOutcome || "结果支持",
    missing: (t as unknown as { evaluationEvidenceMissing?: string }).evaluationEvidenceMissing || "缺失",
    unobserved: (t as unknown as { evaluationEvidenceUnobserved?: string }).evaluationEvidenceUnobserved || "未观察",
    not_applicable: (t as unknown as { evaluationEvidenceNA?: string }).evaluationEvidenceNA || "不适用",
  };
  return labels[state] || state;
}

// ── DimensionScoreCard 组件 ────────────────────────────────────────────────

interface DimensionScoreCardProps {
  score: DimensionScore;
  className?: string;
}

/**
 * 维度评分卡片
 */
export function DimensionScoreCard({
  score,
  className,
}: DimensionScoreCardProps): React.ReactElement {
  const { t } = useI18n();
  const dimension = DIMENSIONS.find((d) => d.id === score.dimensionId);
  if (!dimension) return <></>;

  const scoreColorClass = getScoreColorClass(score.score);
  const scoreBgClass = getScoreBgClass(score.score);

  const dimensionIcons: Record<DimensionId, React.ReactNode> = {
    task_understanding: <Target className="h-5 w-5" />,
    controlled_execution: <Wrench className="h-5 w-5" />,
    change_validation: <CheckCircle2 className="h-5 w-5" />,
    reliable_delivery: <Shield className="h-5 w-5" />,
    learning_capture: <BookOpen className="h-5 w-5" />,
  };

  return (
    <Card className={className}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {dimensionIcons[score.dimensionId]}
            <CardTitle className="text-sm font-medium">
              {dimension.name}
            </CardTitle>
          </div>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <span className={`text-lg font-bold ${scoreColorClass}`}>
                  {score.score}
                </span>
              </TooltipTrigger>
              <TooltipContent>
                <p>
                  {t.evaluationScoreCeiling || "评分上限"}: {score.scoreCeiling}
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        <p className="text-xs text-muted-foreground">{dimension.question}</p>

        <Progress
          value={score.score}
          max={100}
          className="h-2"
          // @ts-expect-error - indicatorClassName is valid for shadcn Progress
          indicatorClassName={scoreBgClass}
        />

        <div className="space-y-1.5">
          {score.checkResults.map((checkResult) => (
            <div
              key={checkResult.checkId}
              className="flex items-center justify-between text-xs"
            >
              <span className="text-muted-foreground">
                {dimension.checks.find((c) => c.id === checkResult.checkId)
                  ?.name || checkResult.checkId}
              </span>
              <Badge variant="outline" className="text-xs">
                {getEvidenceStateLabel(checkResult.evidenceState, t)}
              </Badge>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

// ── FindingItem 组件 ────────────────────────────────────────────────────────

interface FindingItemProps {
  finding: Finding;
}

/**
 * 发现项组件
 */
export function FindingItem({
  finding,
}: FindingItemProps): React.ReactElement {
  const { t } = useI18n();
  const { icon, className: severityClass } = getSeverityStyle(
    finding.severity
  );
  const progressStyle = getRepairProgressStyle(finding.repairProgress);

  return (
    <div className="rounded-lg border p-3 space-y-2">
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-start gap-2">
          <span className={`mt-0.5 ${severityClass} rounded p-1`}>{icon}</span>
          <div className="space-y-1">
            <p className="text-sm font-medium">{finding.description}</p>
            <p className="text-xs text-muted-foreground">{finding.impact}</p>
          </div>
        </div>
        <Badge variant={progressStyle.variant} className={progressStyle.className}>
          {finding.repairProgress === "pending"
            ? t.evaluationRepairPending || "待修复"
            : finding.repairProgress === "verified"
              ? t.evaluationRepairVerified || "已验证"
              : finding.repairProgress === "partial"
                ? t.evaluationRepairPartial || "部分完成"
                : t.evaluationRepairBlocked || "阻塞"}
        </Badge>
      </div>

      <div className="text-xs text-muted-foreground space-y-1">
        <p>
          <strong>{t.evaluationOwner || "所有者"}:</strong> {finding.owner}
        </p>
        <p>
          <strong>{t.evaluationRepairSuggestion || "修复建议"}:</strong>{" "}
          {finding.repairSuggestion}
        </p>
        <p>
          <strong>{t.evaluationValidationRoute || "验证路由"}:</strong>{" "}
          {finding.validationRoute}
        </p>
      </div>
    </div>
  );
}

// ── EvaluationSummaryCard 组件 ─────────────────────────────────────────────

interface EvaluationSummaryCardProps {
  overallScore: number;
  dimensionScores: DimensionScore[];
  findings: Finding[];
  className?: string;
}

/**
 * 评估摘要卡片
 */
export function EvaluationSummaryCard({
  overallScore,
  dimensionScores,
  findings,
  className,
}: EvaluationSummaryCardProps): React.ReactElement {
  const { t } = useI18n();
  const criticalCount = findings.filter((f) => f.severity === "critical").length;
  const majorCount = findings.filter((f) => f.severity === "major").length;
  const pendingCount = findings.filter((f) => f.repairProgress === "pending").length;

  const scoreColorClass = getScoreColorClass(overallScore);

  return (
    <Card className={className}>
      <CardHeader>
        <CardTitle className="text-lg">
          {t.evaluationSummary || "评估摘要"}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* 总体评分 */}
        <div className="flex items-center justify-center">
          <div className="relative">
            <div
              className={`text-5xl font-bold ${scoreColorClass}`}
            >
              {overallScore}
            </div>
            <p className="text-center text-sm text-muted-foreground mt-1">
              {t.evaluationOverallScore || "总体评分"}
            </p>
          </div>
        </div>

        {/* 发现统计 */}
        <div className="grid grid-cols-3 gap-2 text-center">
          <div className="rounded-lg bg-red-50 p-2">
            <p className="text-xl font-bold text-red-500">{criticalCount}</p>
            <p className="text-xs text-muted-foreground">
              {t.evaluationCritical || "严重"}
            </p>
          </div>
          <div className="rounded-lg bg-orange-50 p-2">
            <p className="text-xl font-bold text-orange-500">{majorCount}</p>
            <p className="text-xs text-muted-foreground">
              {t.evaluationMajor || "主要"}
            </p>
          </div>
          <div className="rounded-lg bg-yellow-50 p-2">
            <p className="text-xl font-bold text-yellow-500">{pendingCount}</p>
            <p className="text-xs text-muted-foreground">
              {t.evaluationPendingRepairs || "待修复"}
            </p>
          </div>
        </div>

        {/* 维度评分列表 */}
        <div className="space-y-2">
          <p className="text-sm font-medium">
            {t.evaluationDimensionScores || "维度评分"}
          </p>
          {dimensionScores.map((ds) => (
            <div
              key={ds.dimensionId}
              className="flex items-center justify-between"
            >
              <span className="text-sm text-muted-foreground">
                {DIMENSIONS.find((d) => d.id === ds.dimensionId)?.name}
              </span>
              <span className={`text-sm font-medium ${getScoreColorClass(ds.score)}`}>
                {ds.score}
              </span>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

// ── EvaluationDashboard 组件 ───────────────────────────────────────────────

interface EvaluationDashboardProps {
  overallScore: number;
  dimensionScores: DimensionScore[];
  findings: Finding[];
  className?: string;
}

/**
 * 评估仪表板
 */
export function EvaluationDashboard({
  overallScore,
  dimensionScores,
  findings,
  className,
}: EvaluationDashboardProps): React.ReactElement {
  const { t } = useI18n();

  return (
    <div className={`space-y-4 ${className ?? ""}`}>
      {/* 摘要卡片 */}
      <EvaluationSummaryCard
        overallScore={overallScore}
        dimensionScores={dimensionScores}
        findings={findings}
      />

      {/* 维度评分网格 */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {dimensionScores.map((ds) => (
          <DimensionScoreCard key={ds.dimensionId} score={ds} />
        ))}
      </div>

      {/* 发现列表 */}
      {findings.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">
              {(t as unknown as { evaluationFindings?: string }).evaluationFindings || "发现项"} ({findings.length})
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {findings.map((finding) => (
              <FindingItem key={finding.id} finding={finding} />
            ))}
          </CardContent>
        </Card>
      )}
    </div>
  );
}