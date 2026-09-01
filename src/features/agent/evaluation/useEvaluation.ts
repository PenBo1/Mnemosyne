/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Agent 评估 React Hooks
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useCallback, useRef, useEffect } from "react";
import type {
  DimensionId,
  DimensionScore,
  Finding,
  TaskEpisode,
  EvaluationConfig,
  CheckId,
  FindingSeverity,
  RepairProgress,
} from "./types";
import {
  AgentEvaluator,
  type EvaluationResult,
  type EvidenceCollector,
} from "./evaluator";

// ── useAgentEvaluation Hook ────────────────────────────────────────────────

/**
 * useAgentEvaluation 配置
 */
interface UseAgentEvaluationConfig extends Partial<EvaluationConfig> {
  /** 是否自动开始评估 */
  autoStart?: boolean;
  /** 评估间隔（毫秒） */
  evaluationInterval?: number;
}

/**
 * useAgentEvaluation 返回类型
 */
interface UseAgentEvaluationResult {
  /** 当前任务回合 */
  episode: TaskEpisode | null;
  /** 评估结果 */
  result: EvaluationResult | null;
  /** 是否正在评估 */
  isEvaluating: boolean;
  /** 错误信息 */
  error: string | null;
  /** 开始评估 */
  startEvaluation: (
    goal: string,
    acceptanceBoundary: string,
    sessionId: string,
    workspaceId: string
  ) => void;
  /** 完成评估 */
  completeEvaluation: () => Promise<void>;
  /** 添加发现项 */
  addFinding: (
    checkId: CheckId,
    severity: FindingSeverity,
    description: string,
    impact: string,
    repairSuggestion: string,
    validationRoute: string,
    owner: string
  ) => void;
  /** 更新发现项修复进度 */
  updateFindingProgress: (findingId: string, progress: RepairProgress) => void;
  /** 重置评估 */
  reset: () => void;
}

/**
 * Agent 评估 Hook
 */
export function useAgentEvaluation(
  config: UseAgentEvaluationConfig = {}
): UseAgentEvaluationResult {
  const {
    autoStart = false,
    evaluationInterval,
    ...evaluatorConfig
  } = config;

  const [episode, setEpisode] = useState<TaskEpisode | null>(null);
  const [result, setResult] = useState<EvaluationResult | null>(null);
  const [isEvaluating, setIsEvaluating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const evaluatorRef = useRef<AgentEvaluator | null>(null);

  // 初始化评估器
  useEffect(() => {
    if (!evaluatorRef.current) {
      evaluatorRef.current = new AgentEvaluator(evaluatorConfig);
    }
  }, [evaluatorConfig]);

  // 开始评估
  const startEvaluation = useCallback(
    (
      goal: string,
      acceptanceBoundary: string,
      sessionId: string,
      workspaceId: string
    ) => {
      if (!evaluatorRef.current) return;

      setError(null);
      setResult(null);
      setIsEvaluating(true);

      try {
        const newEpisode = evaluatorRef.current.startEpisode(
          goal,
          acceptanceBoundary,
          sessionId,
          workspaceId
        );
        setEpisode(newEpisode);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setIsEvaluating(false);
      }
    },
    []
  );

  // 完成评估
  const completeEvaluation = useCallback(async () => {
    if (!evaluatorRef.current || !episode) return;

    setIsEvaluating(true);
    setError(null);

    try {
      const evalResult = await evaluatorRef.current.completeEvaluation({
        sessionId: episode.sessionId,
        workspaceId: episode.workspaceId,
        goal: episode.goal,
      });
      setResult(evalResult);
      setEpisode(evalResult.episode);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsEvaluating(false);
    }
  }, [episode]);

  // 添加发现项
  const addFinding = useCallback(
    (
      checkId: CheckId,
      severity: FindingSeverity,
      description: string,
      impact: string,
      repairSuggestion: string,
      validationRoute: string,
      owner: string
    ) => {
      if (!evaluatorRef.current) return;

      try {
        evaluatorRef.current.addFinding(
          checkId,
          severity,
          description,
          impact,
          repairSuggestion,
          validationRoute,
          owner
        );
        setEpisode(evaluatorRef.current.getCurrentEpisode());
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    []
  );

  // 更新发现项修复进度
  const updateFindingProgress = useCallback(
    (findingId: string, progress: RepairProgress) => {
      if (!evaluatorRef.current) return;

      evaluatorRef.current.updateFindingRepairProgress(findingId, progress);
      setEpisode(evaluatorRef.current.getCurrentEpisode());
    },
    []
  );

  // 重置
  const reset = useCallback(() => {
    setEpisode(null);
    setResult(null);
    setError(null);
    setIsEvaluating(false);
  }, []);

  return {
    episode,
    result,
    isEvaluating,
    error,
    startEvaluation,
    completeEvaluation,
    addFinding,
    updateFindingProgress,
    reset,
  };
}

// ── useDimensionScore Hook ──────────────────────────────────────────────────

/**
 * 维度分数监控 Hook
 */
export function useDimensionScore(
  dimensionScores: DimensionScore[]
): Record<DimensionId, number> {
  return dimensionScores.reduce(
    (acc, ds) => {
      acc[ds.dimensionId] = ds.score;
      return acc;
    },
    {} as Record<DimensionId, number>
  );
}

// ── useFindingSummary Hook ──────────────────────────────────────────────────

/**
 * 发现项摘要 Hook
 */
export function useFindingSummary(findings: Finding[]): {
  critical: number;
  major: number;
  minor: number;
  info: number;
  pending: number;
  verified: number;
  blocked: number;
} {
  return {
    critical: findings.filter((f) => f.severity === "critical").length,
    major: findings.filter((f) => f.severity === "major").length,
    minor: findings.filter((f) => f.severity === "minor").length,
    info: findings.filter((f) => f.severity === "info").length,
    pending: findings.filter((f) => f.repairProgress === "pending").length,
    verified: findings.filter((f) => f.repairProgress === "verified").length,
    blocked: findings.filter((f) => f.repairProgress === "blocked").length,
  };
}

// ── useEvaluationCollector Hook ─────────────────────────────────────────────

/**
 * 注册证据收集器的 Hook
 */
export function useEvaluationCollector(
  checkId: CheckId,
  collector: EvidenceCollector
): void {
  const evaluatorRef = useRef<AgentEvaluator | null>(null);

  useEffect(() => {
    if (!evaluatorRef.current) {
      evaluatorRef.current = new AgentEvaluator();
    }

    evaluatorRef.current.registerCollector(checkId, collector);

    return () => {
      // 清理时移除收集器
      // 注意：AgentEvaluator 当前不支持移除收集器
    };
  }, [checkId, collector]);
}