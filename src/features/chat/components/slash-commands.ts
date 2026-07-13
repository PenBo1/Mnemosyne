// 斜杠命令注册表 —— 适配 Mnemosyne 小说创作场景
//
// 命令分类:
//   action     —— 触发即时动作 (新建/清空/续写/导出)
//   mode       —— 切换 AI 工作模式 (角色/世界观/情节)
//   navigation —— 跳转到其他功能页 (Wiki/记忆)
//   setting    —— 调整会话参数 (思考深度)
//
// 每条命令携带 i18n 键，由 SlashCommandMenu 渲染时翻译。
import type { LucideIcon } from "lucide-react";
import {
  Plus,
  Eraser,
  PenLine,
  HelpCircle,
  Activity,
  Gauge,
  Users,
  Globe,
  GitBranch,
  BookOpen,
  Brain,
  Download,
} from "lucide-react";

export type SlashCommandCategory = "action" | "mode" | "navigation" | "setting";

export interface SlashCommand {
  /** 命令词干，含前导斜杠 */
  stem: string;
  /** 是否接受参数 (用于决定补全后是否追加空格) */
  hasArgs: boolean;
  /** 图标 */
  icon: LucideIcon;
  /** i18n 键名 (label) —— 对应 t.agentChat[labelKey] */
  labelKey: string;
  /** i18n 键名 (description) —— 对应 t.agentChat[descKey] */
  descKey: string;
  category: SlashCommandCategory;
}

/** 全部斜杠命令 (按展示顺序) */
export const SLASH_COMMANDS: readonly SlashCommand[] = [
  {
    stem: "/new",
    hasArgs: false,
    icon: Plus,
    labelKey: "slashNew",
    descKey: "slashNewDesc",
    category: "action",
  },
  {
    stem: "/clear",
    hasArgs: false,
    icon: Eraser,
    labelKey: "slashClear",
    descKey: "slashClearDesc",
    category: "action",
  },
  {
    stem: "/write",
    hasArgs: false,
    icon: PenLine,
    labelKey: "slashWrite",
    descKey: "slashWriteDesc",
    category: "action",
  },
  {
    stem: "/help",
    hasArgs: false,
    icon: HelpCircle,
    labelKey: "slashHelp",
    descKey: "slashHelpDesc",
    category: "action",
  },
  {
    stem: "/status",
    hasArgs: false,
    icon: Activity,
    labelKey: "slashStatus",
    descKey: "slashStatusDesc",
    category: "action",
  },
  {
    stem: "/depth",
    hasArgs: true,
    icon: Gauge,
    labelKey: "slashDepth",
    descKey: "slashDepthDesc",
    category: "setting",
  },
  {
    stem: "/character",
    hasArgs: false,
    icon: Users,
    labelKey: "slashCharacter",
    descKey: "slashCharacterDesc",
    category: "mode",
  },
  {
    stem: "/world",
    hasArgs: false,
    icon: Globe,
    labelKey: "slashWorld",
    descKey: "slashWorldDesc",
    category: "mode",
  },
  {
    stem: "/plot",
    hasArgs: false,
    icon: GitBranch,
    labelKey: "slashPlot",
    descKey: "slashPlotDesc",
    category: "mode",
  },
  {
    stem: "/wiki",
    hasArgs: false,
    icon: BookOpen,
    labelKey: "slashWiki",
    descKey: "slashWikiDesc",
    category: "navigation",
  },
  {
    stem: "/memory",
    hasArgs: false,
    icon: Brain,
    labelKey: "slashMemory",
    descKey: "slashMemoryDesc",
    category: "navigation",
  },
  {
    stem: "/export",
    hasArgs: true,
    icon: Download,
    labelKey: "slashExport",
    descKey: "slashExportDesc",
    category: "action",
  },
] as const;

export type SlashNavigationDirection = "up" | "down";

/**
 * 按前缀过滤斜杠命令。
 * 仅当输入以 / 开头时返回建议；输入为空或非斜杠开头返回空数组。
 */
export function getSlashSuggestions(input: string): SlashCommand[] {
  const value = input.trim();
  if (!value.startsWith("/")) return [];

  // 取第一个空白前的词干作为过滤键
  const stem = value.match(/^\/\S*/)?.[0] ?? value;
  return SLASH_COMMANDS.filter((cmd) => cmd.stem.startsWith(stem));
}

/** 循环移动选中索引 (down 正向, up 反向) */
export function getNextSlashSelection(
  currentIndex: number,
  suggestionCount: number,
  direction: SlashNavigationDirection,
): number {
  if (suggestionCount <= 0) return 0;
  if (direction === "down") return (currentIndex + 1) % suggestionCount;
  return (currentIndex - 1 + suggestionCount) % suggestionCount;
}

/**
 * 应用选中的命令到输入框:
 *   - 有参数命令: 插入词干 + 空格 (如 "/depth ")
 *   - 无参数命令: 仅插入词干 (如 "/new")
 */
export function applySlashSuggestion(command: SlashCommand): string {
  return command.hasArgs ? `${command.stem} ` : command.stem;
}
