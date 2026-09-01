/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Agent 评估模型模块导出
 * ═══════════════════════════════════════════════════════════════════════════
 */

// 类型导出
export type {
  EvidenceState,
  DimensionId,
  CheckId,
  Dimension,
  Check,
  CheckResult,
  DimensionScore,
  FindingSeverity,
  RepairProgress,
  Finding,
  TaskEpisode,
  EvaluationConfig,
} from "./types";

// 常量导出
export {
  EVIDENCE_SCORE_CEILING,
  DIMENSIONS,
  DEFAULT_EVALUATION_CONFIG,
  getCheckDefinition,
  getDimensionDefinition,
  getCheckIdsForDimension,
} from "./types";

// 评估器导出
export {
  AgentEvaluator,
  defaultEvaluator,
  createTauriCommandCollector,
  createPresenceCollector,
} from "./evaluator";
export type {
  EvaluationResult,
  EvaluationSummary,
  EvidenceCollectorInput,
  EvidenceCollectorResult,
  EvidenceCollector,
} from "./evaluator";

// Hooks 导出
export {
  useAgentEvaluation,
  useDimensionScore,
  useFindingSummary,
  useEvaluationCollector,
} from "./useEvaluation";

// UI 组件导出
export {
  DimensionScoreCard,
  FindingItem,
  EvaluationSummaryCard,
  EvaluationDashboard,
} from "./EvaluationDisplay";