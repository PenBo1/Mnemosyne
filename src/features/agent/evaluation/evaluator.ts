/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Agent 评估器 - 执行维度评分和证据判断
 * ═══════════════════════════════════════════════════════════════════════════
 */

import type {
  CheckId,
  CheckResult,
  DimensionId,
  DimensionScore,
  EvidenceState,
  Finding,
  FindingSeverity,
  RepairProgress,
  TaskEpisode,
  EvaluationConfig,
} from "./types";
import {
  DIMENSIONS,
  EVIDENCE_SCORE_CEILING,
  getCheckIdsForDimension,
  DEFAULT_EVALUATION_CONFIG,
} from "./types";
import { callIpcCommand } from "../services/evaluation-ipc";

// ── 评估结果类型 ────────────────────────────────────────────────────────────

/**
 * 评估结果
 */
export interface EvaluationResult {
  /** 任务回合 */
  episode: TaskEpisode;
  /** 评估摘要 */
  summary: EvaluationSummary;
  /** 警告列表 */
  warnings: string[];
}

/**
 * 评估摘要
 */
export interface EvaluationSummary {
  /** 总体得分 */
  overallScore: number;
  /** 各维度得分 */
  dimensionScores: Record<DimensionId, number>;
  /** 关键发现数量 */
  criticalFindings: number;
  /** 主要发现数量 */
  majorFindings: number;
  /** 待修复发现数量 */
  pendingRepairs: number;
  /** 评估时间 */
  evaluatedAt: number;
}

// ── 证据收集器 ──────────────────────────────────────────────────────────────

/**
 * 证据收集器输入
 */
export interface EvidenceCollectorInput {
  /** 会话 ID */
  sessionId: string;
  /** 工作区 ID */
  workspaceId: string;
  /** 用户目标 */
  goal: string;
  /** 检查项 ID */
  checkId: CheckId;
}

/**
 * 证据收集器结果
 */
export interface EvidenceCollectorResult {
  /** 证据状态 */
  evidenceState: EvidenceState;
  /** 结果描述 */
  result: string;
  /** 证据引用 */
  evidenceReferences: string[];
}

/**
 * 证据收集器函数类型
 */
export type EvidenceCollector = (
  input: EvidenceCollectorInput
) => Promise<EvidenceCollectorResult>;

// ── AgentEvaluator 类 ───────────────────────────────────────────────────────

/**
 * Agent 评估器
 */
export class AgentEvaluator {
  private config: EvaluationConfig;
  private evidenceCollectors: Map<CheckId, EvidenceCollector>;
  private currentEpisode: TaskEpisode | null = null;

  constructor(config: Partial<EvaluationConfig> = {}) {
    this.config = { ...DEFAULT_EVALUATION_CONFIG, ...config };
    this.evidenceCollectors = new Map();
  }

  /**
   * 注册证据收集器
   */
  registerCollector(checkId: CheckId, collector: EvidenceCollector): void {
    this.evidenceCollectors.set(checkId, collector);
  }

  /**
   * 开始新的任务回合评估
   */
  startEpisode(
    goal: string,
    acceptanceBoundary: string,
    sessionId: string,
    workspaceId: string
  ): TaskEpisode {
    const episode: TaskEpisode = {
      id: crypto.randomUUID(),
      goal,
      acceptanceBoundary,
      startedAt: Date.now(),
      sessionId,
      workspaceId,
      dimensionScores: [],
      findings: [],
      overallScore: 0,
      status: "in_progress",
    };
    this.currentEpisode = episode;
    return episode;
  }

  /**
   * 评估单个维度
   */
  async evaluateDimension(
    dimensionId: DimensionId,
    input: Omit<EvidenceCollectorInput, "checkId">
  ): Promise<DimensionScore> {
    const checkIds = getCheckIdsForDimension(dimensionId);
    const checkResults: CheckResult[] = [];
    let minEvidenceState: EvidenceState = "outcome_supported";
    let hasBlockedCheck = false;

    for (const checkId of checkIds) {
      const result = await this.evaluateCheck({ ...input, checkId });
      checkResults.push(result);

      // 更新最低证据状态
      if (
        EVIDENCE_SCORE_CEILING[result.evidenceState] <
        EVIDENCE_SCORE_CEILING[minEvidenceState]
      ) {
        minEvidenceState = result.evidenceState;
      }

      // 检查是否有阻塞的检查项
      if (
        result.evidenceState === "missing" ||
        result.evidenceState === "unobserved"
      ) {
        hasBlockedCheck = true;
      }
    }

    // 计算评分
    const scoreCeiling = EVIDENCE_SCORE_CEILING[minEvidenceState];
    let score = hasBlockedCheck ? Math.min(59, scoreCeiling) : scoreCeiling;

    // 如果有发现项，降低评分
    const criticalFindings = this.currentEpisode?.findings.filter(
      (f) =>
        checkIds.includes(f.checkId) &&
        f.severity === "critical" &&
        f.repairProgress !== "verified"
    ).length ?? 0;

    if (criticalFindings > 0) {
      score = Math.min(score - criticalFindings * 10, 59);
    }

    return {
      dimensionId,
      score: Math.max(0, score),
      scoreCeiling,
      checkResults,
      timestamp: Date.now(),
    };
  }

  /**
   * 评估单个检查项
   */
  private async evaluateCheck(
    input: EvidenceCollectorInput
  ): Promise<CheckResult> {
    const collector = this.evidenceCollectors.get(input.checkId);

    if (!collector) {
      // 默认返回 Unobserved
      return {
        checkId: input.checkId,
        evidenceState: "unobserved",
        result: "未配置证据收集器",
        evidenceReferences: [],
        findingReferences: [],
        timestamp: Date.now(),
      };
    }

    try {
      const result = await Promise.race([
        collector(input),
        this.createTimeout(input.checkId),
      ]);
      return {
        checkId: input.checkId,
        evidenceState: result.evidenceState,
        result: result.result,
        evidenceReferences: result.evidenceReferences,
        findingReferences: [],
        timestamp: Date.now(),
      };
    } catch (error) {
      return {
        checkId: input.checkId,
        evidenceState: "unobserved",
        result: `证据收集失败: ${error}`,
        evidenceReferences: [],
        findingReferences: [],
        timestamp: Date.now(),
      };
    }
  }

  /**
   * 创建超时 Promise
   */
  private createTimeout(checkId: CheckId): Promise<never> {
    return new Promise((_, reject) => {
      setTimeout(
        () => reject(new Error(`证据收集超时: ${checkId}`)),
        this.config.evidenceTimeoutMs
      );
    });
  }

  /**
   * 完成评估并生成报告
   */
  async completeEvaluation(
    input: Omit<EvidenceCollectorInput, "checkId">
  ): Promise<EvaluationResult> {
    if (!this.currentEpisode) {
      throw new Error("没有正在进行的任务回合");
    }

    const warnings: string[] = [];
    const dimensionScores: DimensionScore[] = [];

    // 评估所有维度
    for (const dimension of DIMENSIONS) {
      const score = await this.evaluateDimension(dimension.id, input);
      dimensionScores.push(score);
    }

    // 计算总体分数
    const overallScore = Math.round(
      dimensionScores.reduce((sum, ds) => sum + ds.score, 0) /
        dimensionScores.length
    );

    // 更新任务回合
    this.currentEpisode.dimensionScores = dimensionScores;
    this.currentEpisode.overallScore = overallScore;
    this.currentEpisode.completedAt = Date.now();
    this.currentEpisode.status = "completed";

    // 生成摘要
    const summary: EvaluationSummary = {
      overallScore,
      dimensionScores: dimensionScores.reduce(
        (acc, ds) => {
          acc[ds.dimensionId] = ds.score;
          return acc;
        },
        {} as Record<DimensionId, number>
      ),
      criticalFindings: this.currentEpisode.findings.filter(
        (f) => f.severity === "critical"
      ).length,
      majorFindings: this.currentEpisode.findings.filter(
        (f) => f.severity === "major"
      ).length,
      pendingRepairs: this.currentEpisode.findings.filter(
        (f) => f.repairProgress === "pending"
      ).length,
      evaluatedAt: Date.now(),
    };

    const result: EvaluationResult = {
      episode: this.currentEpisode,
      summary,
      warnings,
    };

    this.currentEpisode = null;
    return result;
  }

  /**
   * 添加发现项
   */
  addFinding(
    checkId: CheckId,
    severity: FindingSeverity,
    description: string,
    impact: string,
    repairSuggestion: string,
    validationRoute: string,
    owner: string
  ): Finding {
    if (!this.currentEpisode) {
      throw new Error("没有正在进行的任务回合");
    }

    const finding: Finding = {
      id: crypto.randomUUID(),
      checkId,
      severity,
      description,
      impact,
      repairSuggestion,
      validationRoute,
      owner,
      repairProgress: "pending",
      timestamp: Date.now(),
    };

    this.currentEpisode.findings.push(finding);
    return finding;
  }

  /**
   * 更新发现项修复进度
   */
  updateFindingRepairProgress(
    findingId: string,
    progress: RepairProgress
  ): void {
    if (!this.currentEpisode) return;

    const finding = this.currentEpisode.findings.find(
      (f) => f.id === findingId
    );
    if (finding) {
      finding.repairProgress = progress;
    }
  }

  /**
   * 获取当前任务回合
   */
  getCurrentEpisode(): TaskEpisode | null {
    return this.currentEpisode;
  }
}

// ── 预定义证据收集器 ────────────────────────────────────────────────────────

/**
 * 创建基于 Tauri 命令的证据收集器
 */
export function createTauriCommandCollector(
  _checkId: CheckId,
  commandName: string,
  mapResult: (data: unknown) => EvidenceCollectorResult
): EvidenceCollector {
  return async (input: EvidenceCollectorInput): Promise<EvidenceCollectorResult> => {
    try {
      const data = await callIpcCommand(commandName, {
        sessionId: input.sessionId,
        workspaceId: input.workspaceId,
        goal: input.goal,
      });
      return mapResult(data);
    } catch (error) {
      return {
        evidenceState: "unobserved",
        result: `命令调用失败: ${error}`,
        evidenceReferences: [],
      };
    }
  };
}

/**
 * 创建简单的存在性检查收集器
 */
export function createPresenceCollector(
  _checkId: CheckId,
  check: () => Promise<boolean>,
  presentResult: string,
  missingResult: string
): EvidenceCollector {
  return async (): Promise<EvidenceCollectorResult> => {
    try {
      const isPresent = await check();
      return {
        evidenceState: isPresent ? "present" : "missing",
        result: isPresent ? presentResult : missingResult,
        evidenceReferences: [],
      };
    } catch {
      return {
        evidenceState: "unobserved",
        result: "检查失败",
        evidenceReferences: [],
      };
    }
  };
}

// ── 单例实例 ────────────────────────────────────────────────────────────────

/**
 * 默认评估器实例
 */
export const defaultEvaluator = new AgentEvaluator();