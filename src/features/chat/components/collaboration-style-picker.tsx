/**
 * ═══════════════════════════════════════════════════════════════════════════
 * CollaborationStylePicker - 协作风格选择器组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

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

// ── 常量配置 ────────────────────────────────────────────────────────────────

const STYLE_COLOR: Record<CollaborationStyle, string> = {
  efficient: "text-emerald-500",
  thoughtful: "text-blue-500",
  patient: "text-amber-500",
  decisive: "text-rose-500",
};

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 检查是否为有效的协作风格值
 */
function isStyleValue(s: string): boolean {
  return (
    s === "efficient" ||
    s === "thoughtful" ||
    s === "patient" ||
    s === "decisive"
  );
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 协作风格选择器，下拉选择 Agent 的回复风格
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

  /**
   * 处理风格变更
   */
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