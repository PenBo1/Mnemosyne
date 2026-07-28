/**
 * ═══════════════════════════════════════════════════════════════════════════
 * EditModelDialog - 编辑 AI 模型对话框
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Loader2Icon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { useModelSettings } from "@/features/settings/hooks";
import type { AiModelConfig } from "@/services/settings";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface EditModelDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  model: AiModelConfig | null;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 编辑 AI 模型对话框，用于修改已存在的模型配置
 */
export function EditModelDialog({ open, onOpenChange, model }: EditModelDialogProps) {
  const { t } = useI18n();
  const { updateModel } = useModelSettings();
  const [name, setName] = useState(model?.name ?? "");
  const [modelName, setModelName] = useState(model?.model ?? "");
  const [apiKey, setApiKey] = useState(model?.api_key ?? "");
  const [baseUrl, setBaseUrl] = useState(model?.base_url ?? "");
  const [saving, setSaving] = useState(false);

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  /**
   * 保存模型配置
   */
  async function handleSave() {
    if (!model || !name.trim()) return;
    setSaving(true);
    try {
      await updateModel(model.id, {
        name,
        model: modelName,
        api_key: apiKey,
        base_url: baseUrl,
      });
      onOpenChange(false);
    } catch {
      // 错误由 hook 处理
    } finally {
      setSaving(false);
    }
  }

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t.settings.modelSettings.editProvider}</DialogTitle>
          <DialogDescription>{t.settings.modelSettings.subtitle}</DialogDescription>
        </DialogHeader>
        <FieldGroup>
          {/* ── 配置名称 ──────────────────────────────────────────────────── */}
          <Field>
            <FieldLabel>{t.agents.name}</FieldLabel>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My OpenAI Config"
            />
          </Field>
          {/* ── 模型名称 ──────────────────────────────────────────────────── */}
          <Field>
            <FieldLabel>{t.settings.modelSettings.model}</FieldLabel>
            <Input
              value={modelName}
              onChange={(e) => setModelName(e.target.value)}
              placeholder={t.settings.modelSettings.modelPlaceholder}
            />
          </Field>
          {/* ── API 密钥 ──────────────────────────────────────────────────── */}
          <Field>
            <FieldLabel>{t.settings.modelSettings.apiKey}</FieldLabel>
            <Input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={t.settings.modelSettings.apiKeyPlaceholder}
            />
          </Field>
          {/* ── 基础 URL ──────────────────────────────────────────────────── */}
          <Field>
            <FieldLabel>{t.settings.modelSettings.baseUrl}</FieldLabel>
            <Input
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder={t.settings.modelSettings.baseUrlPlaceholder}
            />
          </Field>
        </FieldGroup>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t.settings.modelSettings.cancel}
          </Button>
          <Button onClick={handleSave} disabled={!name.trim() || saving}>
            {saving ? <Loader2Icon className="size-4 animate-spin" data-icon="inline-start" /> : null}
            {t.settings.modelSettings.save}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}