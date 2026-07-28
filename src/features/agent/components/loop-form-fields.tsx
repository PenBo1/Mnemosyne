/**
 * ═══════════════════════════════════════════════════════════════════════════
 * LoopFormFields - 循环任务表单字段组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo } from "react";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Clock, Zap } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type { TriggerType } from "@/features/loop/types";

// ── 类型定义 ────────────────────────────────────────────────────────────────

export interface LoopFormData {
  name: string;
  triggerType: TriggerType;
  intervalMs: number;
  eventType: string;
  eventFilter: string;
  maxIterations: number;
}

interface LoopFormFieldsProps {
  formData: LoopFormData;
  onChange: (data: Partial<LoopFormData>) => void;
}

// ── 常量配置 ────────────────────────────────────────────────────────────────

export const DEFAULT_FORM_DATA: LoopFormData = {
  name: "",
  triggerType: "timer",
  intervalMs: 60000,
  eventType: "",
  eventFilter: "",
  maxIterations: 10,
};

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 循环任务表单字段组件，用于创建和编辑循环任务
 */
export const LoopFormFields = memo(function LoopFormFields({ formData, onChange }: LoopFormFieldsProps) {
  const { t } = useI18n();

  return (
    <div className="flex flex-col gap-3">
      <div>
        <label className="text-xs text-muted-foreground mb-1 block">
          {t.loopPanel.name}
        </label>
        <Input
          value={formData.name}
          onChange={(e) => onChange({ name: e.target.value })}
          placeholder={t.loopPanel.namePlaceholder}
          className="text-xs"
        />
      </div>

      <div>
        <label className="text-xs text-muted-foreground mb-1 block">
          {t.loopPanel.triggerType}
        </label>
        <Select
          value={formData.triggerType}
          onValueChange={(v) => onChange({ triggerType: v as TriggerType })}
        >
          <SelectTrigger className="text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="timer">
              <div className="flex items-center gap-1.5">
                <Clock className="size-3" />
                {t.loopPanel.timer}
              </div>
            </SelectItem>
            <SelectItem value="event">
              <div className="flex items-center gap-1.5">
                <Zap className="size-3" />
                {t.loopPanel.event}
              </div>
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      {formData.triggerType === "timer" && (
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">
            {t.loopPanel.intervalMs}
          </label>
          <Input
            type="number"
            value={formData.intervalMs}
            onChange={(e) => onChange({ intervalMs: parseInt(e.target.value) || 60000 })}
            className="text-xs"
            min={1000}
          />
        </div>
      )}

      {formData.triggerType === "event" && (
        <>
          <div>
            <label className="text-xs text-muted-foreground mb-1 block">
              {t.loopPanel.eventType}
            </label>
            <Select
              value={formData.eventType}
              onValueChange={(v) => onChange({ eventType: v })}
            >
              <SelectTrigger className="text-xs">
                <SelectValue placeholder={t.loopPanel.selectEventType} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="file_change">{t.loop.tools.fileChange}</SelectItem>
                <SelectItem value="git_commit">{t.loop.tools.gitCommit}</SelectItem>
                <SelectItem value="chapter_complete">{t.loop.tools.chapterComplete}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div>
            <label className="text-xs text-muted-foreground mb-1 block">
              {t.loopPanel.eventFilter}
            </label>
            <Input
              value={formData.eventFilter}
              onChange={(e) => onChange({ eventFilter: e.target.value })}
              placeholder={t.loopPanel.eventFilterPlaceholder}
              className="text-xs"
            />
          </div>
        </>
      )}

      <div>
        <label className="text-xs text-muted-foreground mb-1 block">
          {t.loopPanel.maxIterations}
        </label>
        <Input
          type="number"
          value={formData.maxIterations}
          onChange={(e) => onChange({ maxIterations: parseInt(e.target.value) || 10 })}
          className="text-xs"
          min={1}
          max={1000}
        />
      </div>
    </div>
  );
});