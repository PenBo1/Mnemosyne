/**
 * ═══════════════════════════════════════════════════════════════════════════
 * EffortPicker - Agent 投入程度选择器组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo, useState, useEffect } from "react";
import { GaugeIcon } from "lucide-react";
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
  getCurrentEffort,
  setCurrentEffort,
} from "@/features/agent/services/llm/chat-runtime";
import { EFFORT_OPTIONS, type EffortLevel } from "@/types/effort";

// ── 常量配置 ────────────────────────────────────────────────────────────────

const EFFORT_COLOR: Record<EffortLevel, string> = {
  low: "text-emerald-500",
  medium: "text-blue-500",
  high: "text-amber-500",
  ultra: "text-violet-500",
};

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 检查是否为有效的投入程度值
 */
function isEffortValue(s: string): boolean {
  return s === "low" || s === "medium" || s === "high" || s === "ultra";
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * Agent 投入程度选择器，下拉选择 Agent 的工作投入级别
 */
export const EffortPicker = memo(function EffortPicker() {
  const { t } = useI18n();
  const [effort, setEffort] = useState<EffortLevel>(() => getCurrentEffort());

  // 同步外部变更(如其他组件修改了 effort)
  useEffect(() => {
    const sync = () => setEffort(getCurrentEffort());
    window.addEventListener("storage", sync);
    return () => window.removeEventListener("storage", sync);
  }, []);

  /**
   * 处理投入程度变更
   */
  const handleChange = (value: string) => {
    if (!isEffortValue(value)) return;
    const next = value as EffortLevel;
    setEffort(next);
    setCurrentEffort(next);
  };

  const currentLabel = t.agentChat.effortOptions[effort].label;
  const colorClass = EFFORT_COLOR[effort];

  return (
    <Select value={effort} onValueChange={handleChange}>
      <SelectTrigger size="sm" className="min-w-28 max-w-40" aria-label={t.agentChat.effortLabel}>
        <GaugeIcon className={colorClass} />
        <SelectValue placeholder={t.agentChat.effortLabel}>
          {currentLabel}
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectLabel>{t.agentChat.effortLabel}</SelectLabel>
          {EFFORT_OPTIONS.map((opt) => (
            <SelectItem key={opt.value} value={opt.value}>
              <span className={EFFORT_COLOR[opt.value]}>
                {t.agentChat.effortOptions[opt.value].label}
              </span>
              <span className="ml-1 text-xs text-muted-foreground">
                · {t.agentChat.effortOptions[opt.value].description}
              </span>
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
});