/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Agent 评估模型类型定义
 * ═══════════════════════════════════════════════════════════════════════════
 */

// ── 证据状态 ────────────────────────────────────────────────────────────────

/**
 * 证据状态枚举
 * - Present: 机制或合约存在
 * - Wired: 可到达相关任务、触发器或所有者路由
 * - Exercised: 已被使用并保留结果
 * - OutcomeSupported: 可比的后续结果支持声明效果
 * - Missing: 必要机制或结果缺失
 * - Unobserved: 可用观察边界无法判定
 * - NotApplicable: 不适用于当前任务
 */
export type EvidenceState =
  | "present"
  | "wired"
  | "exercised"
  | "outcome_supported"
  | "missing"
  | "unobserved"
  | "not_applicable";

/**
 * 证据状态到评分上限的映射
 */
export const EVIDENCE_SCORE_CEILING: Record<EvidenceState, number> = {
  missing: 59,
  unobserved: 59,
  not_applicable: 59,
  present: 74,
  wired: 84,
  exercised: 94,
  outcome_supported: 100,
};

// ── 评估维度 ────────────────────────────────────────────────────────────────

/**
 * 评估维度 ID
 */
export type DimensionId =
  | "task_understanding"
  | "controlled_execution"
  | "change_validation"
  | "reliable_delivery"
  | "learning_capture";

/**
 * 评估维度定义
 */
export interface Dimension {
  /** 维度 ID */
  id: DimensionId;
  /** 维度名称 */
  name: string;
  /** 维度描述 */
  description: string;
  /** 读者问题 */
  question: string;
  /** 该维度下的检查项 */
  checks: Check[];
}

/**
 * 检查项 ID
 */
export type CheckId =
  | "goal_understanding"
  | "relevant_context"
  | "scope_boundary"
  | "instruction_led_start"
  | "supported_operation"
  | "permission_boundary"
  | "relevant_check"
  | "failure_repair"
  | "validate_again"
  | "acceptance_evidence"
  | "high_risk_approval"
  | "rollback_recovery"
  | "lifecycle_repeat_detection"
  | "loop_engineering"
  | "later_validation";

/**
 * 检查项定义
 */
export interface Check {
  /** 检查项 ID */
  id: CheckId;
  /** 检查项名称 */
  name: string;
  /** 检查项描述 */
  description: string;
  /** 所属维度 ID */
  dimensionId: DimensionId;
}

/**
 * 检查项结果
 */
export interface CheckResult {
  /** 检查项 ID */
  checkId: CheckId;
  /** 证据状态 */
  evidenceState: EvidenceState;
  /** 结果描述 */
  result: string;
  /** 证据引用列表 */
  evidenceReferences: string[];
  /** 发现引用列表 */
  findingReferences: string[];
  /** 时间戳 */
  timestamp: number;
}

/**
 * 维度评分结果
 */
export interface DimensionScore {
  /** 维度 ID */
  dimensionId: DimensionId;
  /** 评分（0-100） */
  score: number;
  /** 评分上限（基于证据状态） */
  scoreCeiling: number;
  /** 各检查项结果 */
  checkResults: CheckResult[];
  /** 时间戳 */
  timestamp: number;
}

// ── 发现与修复 ──────────────────────────────────────────────────────────────

/**
 * 发现严重程度
 */
export type FindingSeverity = "critical" | "major" | "minor" | "info";

/**
 * 修复进度状态
 */
export type RepairProgress = "pending" | "verified" | "partial" | "blocked";

/**
 * 发现项定义
 */
export interface Finding {
  /** 发现 ID */
  id: string;
  /** 关联的检查项 ID */
  checkId: CheckId;
  /** 严重程度 */
  severity: FindingSeverity;
  /** 发现描述 */
  description: string;
  /** 影响范围 */
  impact: string;
  /** 最小修复建议 */
  repairSuggestion: string;
  /** 验证路由 */
  validationRoute: string;
  /** 所有者 */
  owner: string;
  /** 修复进度 */
  repairProgress: RepairProgress;
  /** 时间戳 */
  timestamp: number;
}

// ── 任务回合 ────────────────────────────────────────────────────────────────

/**
 * 任务回合定义
 * 一个用户目标及其验收边界的完整周期
 */
export interface TaskEpisode {
  /** 回合 ID */
  id: string;
  /** 用户目标描述 */
  goal: string;
  /** 验收边界 */
  acceptanceBoundary: string;
  /** 开始时间 */
  startedAt: number;
  /** 结束时间（如已完成） */
  completedAt?: number;
  /** 会话 ID */
  sessionId: string;
  /** 工作区 ID */
  workspaceId: string;
  /** 维度评分列表 */
  dimensionScores: DimensionScore[];
  /** 发现列表 */
  findings: Finding[];
  /** 总体评估分数 */
  overallScore: number;
  /** 评估状态 */
  status: "in_progress" | "completed" | "failed";
}

// ── 评估配置 ────────────────────────────────────────────────────────────────

/**
 * 评估配置
 */
export interface EvaluationConfig {
  /** 是否启用核心路径可观测性检查 */
  enableCoreObservability: boolean;
  /** 是否启用学习捕获检查 */
  enableLearningCapture: boolean;
  /** 最大发现数量 */
  maxFindings: number;
  /** 证据超时时间（毫秒） */
  evidenceTimeoutMs: number;
}

/**
 * 默认评估配置
 */
export const DEFAULT_EVALUATION_CONFIG: EvaluationConfig = {
  enableCoreObservability: true,
  enableLearningCapture: true,
  maxFindings: 50,
  evidenceTimeoutMs: 30000,
};

// ── 维度定义常量 ────────────────────────────────────────────────────────────

/**
 * 所有评估维度定义
 */
export const DIMENSIONS: Dimension[] = [
  {
    id: "task_understanding",
    name: "任务理解",
    description: "Agent 是否理解预期结果、应用相关权威上下文、并在显式范围和效果边界内工作",
    question:
      "Agent 是否理解预期结果、应用相关权威上下文、并保持在显式范围和效果边界内？",
    checks: [
      {
        id: "goal_understanding",
        name: "意图与验收",
        description: "任务保留预期结果、完成定义、排除项、更正和未解决问题作为一个可恢复的验收边界",
        dimensionId: "task_understanding",
      },
      {
        id: "relevant_context",
        name: "相关上下文",
        description: "决策使用权威指令、架构或领域所有者、规范来源和重要依赖合约，而非广泛或偶然的上下文",
        dimensionId: "task_understanding",
      },
      {
        id: "scope_boundary",
        name: "范围边界",
        description: "预期文件、模块、生成产物、可见或外部效果、风险、排除项和批准的范围扩展保持显式和可追溯",
        dimensionId: "task_understanding",
      },
    ],
  },
  {
    id: "controlled_execution",
    name: "受控执行",
    description: "Agent 是否能通过支持的路由启动和操作项目，同时保持在强制的权限和操作边界内",
    question:
      "Agent 是否能通过支持的路由启动和操作项目，同时保持在强制的权限和操作边界内？",
    checks: [
      {
        id: "instruction_led_start",
        name: "可复现启动",
        description: "通过项目拥有的非交互式设置和启动路由，从干净或显式声明的起始状态变为可用状态",
        dimensionId: "controlled_execution",
      },
      {
        id: "supported_operation",
        name: "支持的操作",
        description:
          "目标行为通过支持的命令、技能、CLI、Agent、插件或 MCP 支持的工作流可发现和可调用",
        dimensionId: "controlled_execution",
      },
      {
        id: "permission_boundary",
        name: "权限边界",
        description: "文件系统、网络、工具、凭证、外部写入、共享状态和受保护操作保持在强制的授权和清理边界内",
        dimensionId: "controlled_execution",
      },
    ],
  },
  {
    id: "change_validation",
    name: "变更验证",
    description: "Agent 是否运行与最终变更相关的验证、诊断和修复失败、并重新验证修复结果",
    question:
      "Agent 是否运行与最终变更相关的验证、诊断和修复失败、并重新验证修复结果？",
    checks: [
      {
        id: "relevant_check",
        name: "相关验证",
        description: "最终实质性变更被映射到并执行最小项目拥有的检查，直接覆盖其行为、不变量和风险",
        dimensionId: "change_validation",
      },
      {
        id: "failure_repair",
        name: "失败诊断与修复",
        description: "观察到的验证失败被复现、用可归因诊断定位、用因果假说解释、并在最小正确所有者处修复",
        dimensionId: "change_validation",
      },
      {
        id: "validate_again",
        name: "修复后重新验证",
        description: "相同失败的检查或具有相同行为和范围的合理等效检查在修复后的最终状态上再次运行",
        dimensionId: "change_validation",
      },
    ],
  },
  {
    id: "reliable_delivery",
    name: "可靠交付",
    description: "当前结果是否在真实交付边界被接受、具有风险适当的批准和可用的回滚或恢复路径",
    question:
      "当前结果是否在真实交付边界被接受、具有风险适当的批准和可用的回滚或恢复路径？",
    checks: [
      {
        id: "acceptance_evidence",
        name: "交付验收",
        description: "当前结果达到项目的真实审查、所需 CI、合并、发布、部署或等效验收边界，并带有修订绑定的决策证据",
        dimensionId: "reliable_delivery",
      },
      {
        id: "high_risk_approval",
        name: "高风险批准",
        description:
          "每个适用的破坏性、特权、外部、不可逆、共享状态、凭证、发布或生产操作在效果前收到所需决策",
        dimensionId: "reliable_delivery",
      },
      {
        id: "rollback_recovery",
        name: "回滚或恢复",
        description: "实际副作用具有风险比例的回滚、恢复、重试、补偿、幂等重放、安全中止或证明无持久效果的路径",
        dimensionId: "reliable_delivery",
      },
    ],
  },
  {
    id: "learning_capture",
    name: "学习捕获",
    description: "Harness 是否检测生命周期、循环和维护机会、将支持的机会转化为可重用的改进、并保持这些改进随时间准确有效",
    question:
      "Harness 是否检测机会、将支持的机会转化为可重用改进、并保持这些改进随时间准确有效？",
    checks: [
      {
        id: "lifecycle_repeat_detection",
        name: "生命周期机会检测",
        description: "Harness 检测生命周期、循环和维护机会，并将其路由到适当的所有者",
        dimensionId: "learning_capture",
      },
      {
        id: "loop_engineering",
        name: "循环工程",
        description: "支持的机会被转化为可重用的改进，具有持久所有者和验证路由",
        dimensionId: "learning_capture",
      },
      {
        id: "later_validation",
        name: "纵向验证",
        description: "改进在后续可比较结果中得到验证，并保持准确有效",
        dimensionId: "learning_capture",
      },
    ],
  },
];

/**
 * 获取检查项定义
 */
export function getCheckDefinition(checkId: CheckId): Check | undefined {
  for (const dimension of DIMENSIONS) {
    const check = dimension.checks.find((c) => c.id === checkId);
    if (check) return check;
  }
  return undefined;
}

/**
 * 获取维度定义
 */
export function getDimensionDefinition(
  dimensionId: DimensionId
): Dimension | undefined {
  return DIMENSIONS.find((d) => d.id === dimensionId);
}

/**
 * 获取维度下的所有检查项 ID
 */
export function getCheckIdsForDimension(dimensionId: DimensionId): CheckId[] {
  const dimension = getDimensionDefinition(dimensionId);
  return dimension ? dimension.checks.map((c) => c.id) : [];
}