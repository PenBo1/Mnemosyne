// Collaboration Style Picker —— 协作风格选择器。
//
// 四档(对齐 Rust 的 CollaborationStyle enum):
// - efficient:高效极简 —— 简洁直接,聚焦解决问题
// - thoughtful:深思熟虑 —— 充分分析,权衡取舍
// - patient:温和耐心 —— 循序渐进,解释原理
// - decisive:果断执行 —— 行动导向,快速决策
//
// 与 Effort 正交:Effort 控制"做多少",Style 控制"怎么做"
//
// 自包含:
// - 从 chat-runtime 读取/写入 currentCollaborationStyle
// - localStorage 持久化(跨 session 保留)

import { memo, useState, useEffect } from "react";
import { SparklesIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  getCurrentCollaborationStyle,
  setCurrentCollaborationStyle,
} from "@/features/agent/services/llm/chat-runtime";
import {
  COLLABORATION_STYLE_OPTIONS,
  type CollaborationStyle,
} from "@/types/collaboration-style";

const STYLE_COLOR: Record<CollaborationStyle, string> = {
  efficient: "text-emerald-500",
  thoughtful: "text-blue-500",
  patient: "text-amber-500",
  decisive: "text-rose-500",
};

/**
 * 协作风格选择器:下拉选择 Agent 的回复风格。
 *
 * 选择后立即通过 `setCurrentCollaborationStyle` 写入 chat-runtime 模块状态,
 * 下次 sendMessage 时会读取最新值传给 Rust。
 */
export const CollaborationStylePicker = memo(function CollaborationStylePicker() {
  const { t } = useI18n();
  const [style, setStyle] = useState<CollaborationStyle>(getCurrentCollaborationStyle());

  // 同步外部变更(如其他组件修改了 style)
  useEffect(() => {
    const sync = () => setStyle(getCurrentCollaborationStyle());
    window.addEventListener("storage", sync);
    return () => window.removeEventListener("storage", sync);
  }, []);

  const handleChange = (value: string) => {
    if (!isStyleValue(value)) return;
    const next = value as CollaborationStyle;
    setStyle(next);
    setCurrentCollaborationStyle(next);
  };

  const currentLabel = t.agentChat.collaborationStyleOptions[style].label;
  const colorClass = STYLE_COLOR[style];

  return (
    <Select value={style} onValueChange={handleChange}>
      <SelectTrigger size="sm" className="min-w-28 max-w-40" aria-label={t.agentChat.collaborationStyleLabel}>
        <SparklesIcon className={colorClass} />
        <SelectValue placeholder={t.agentChat.collaborationStyleLabel}>
          {currentLabel}
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectLabel>{t.agentChat.collaborationStyleLabel}</SelectLabel>
          {COLLABORATION_STYLE_OPTIONS.map((opt) => (
            <SelectItem key={opt.value} value={opt.value}>
              <span className={STYLE_COLOR[opt.value]}>
                {t.agentChat.collaborationStyleOptions[opt.value].label}
              </span>
              <span className="ml-1 text-xs text-muted-foreground">
                · {t.agentChat.collaborationStyleOptions[opt.value].description}
              </span>
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
});

function isStyleValue(s: string): boolean {
  return (
    s === "efficient" ||
    s === "thoughtful" ||
    s === "patient" ||
    s === "decisive"
  );
}
