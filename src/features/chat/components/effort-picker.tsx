// Effort Picker —— Agent 投入程度选择器。
//
// - Model 决定"能力"(会不会),Effort 决定"态度"(愿不愿意努力做)
// - 小模型 + 高 Effort 可能比大模型 + 低 Effort 效果更好
//
// 四档(对齐 Rust 的 EffortLevel enum):
// - low:快速回复,最小工具调用(5 轮/10 文件/2k tokens)
// - medium:默认,平衡(20 轮/50 文件/8k tokens)
// - high:深度分析,多轮验证(50 轮/200 文件/16k tokens)
// - ultra:ultracode,多 agent 并行(100 轮/1000 文件/32k tokens)
//
// 自包含:
// - 从 chat-runtime 读取/写入 currentEffort
// - localStorage 持久化(跨 session 保留)

import { useState, useEffect } from "react";
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

const EFFORT_COLOR: Record<EffortLevel, string> = {
  low: "text-emerald-500",
  medium: "text-blue-500",
  high: "text-amber-500",
  ultra: "text-violet-500",
};

/**
 * Effort 选择器:下拉选择 Agent 投入程度。
 *
 * 选择后立即通过 `setCurrentEffort` 写入 chat-runtime 模块状态,
 * 下次 sendMessage 时会读取最新值传给 Rust。
 */
export function EffortPicker() {
  const { t } = useI18n();
  const [effort, setEffort] = useState<EffortLevel>(getCurrentEffort());

  // 同步外部变更(如其他组件修改了 effort)
  useEffect(() => {
    const sync = () => setEffort(getCurrentEffort());
    window.addEventListener("storage", sync);
    return () => window.removeEventListener("storage", sync);
  }, []);

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
}

function isEffortValue(s: string): boolean {
  return s === "low" || s === "medium" || s === "high" || s === "ultra";
}
