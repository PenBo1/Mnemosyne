/**
 * ═══════════════════════════════════════════════════════════════════════════
 * AddModelDialog - 添加 AI 模型对话框
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel, FieldSeparator } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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

// ── 常量配置 ────────────────────────────────────────────────────────────────

/** 支持的 AI 提供商列表 */
const PROVIDER_OPTIONS = ["openai", "ollama", "agnes"] as const;

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface AddModelDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 添加 AI 模型对话框，用于配置新的模型提供商
 */
export function AddModelDialog({ open, onOpenChange }: AddModelDialogProps) {
  const { t } = useI18n();
  const { addModel } = useModelSettings();
  const [provider, setProvider] = useState<string>("openai");
  const [name, setName] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [saving, setSaving] = useState(false);

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  /**
   * 保存模型配置
   */
  async function handleSave() {
    if (!apiKey.trim() || !name.trim()) return;
    setSaving(true);
    try {
      await addModel({
        name,
        provider,
        model: model || (provider === "openai" ? "gpt-4o" : provider === "ollama" ? "llama3.1" : "agnes-default"),
        api_key: apiKey,
        base_url: baseUrl || "",
      });
      onOpenChange(false);
      setName("");
      setModel("");
      setApiKey("");
      setBaseUrl("");
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
          <DialogTitle>{t.settings.modelSettings.addProvider}</DialogTitle>
          <DialogDescription>{t.settings.modelSettings.subtitle}</DialogDescription>
        </DialogHeader>
        <FieldGroup>
          {/* ── 提供商选择 ──────────────────────────────────────────────────── */}
          <Field>
            <FieldLabel>{t.settings.modelSettings.provider}</FieldLabel>
            <Select value={provider} onValueChange={setProvider}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PROVIDER_OPTIONS.map((p) => (
                  <SelectItem key={p} value={p}>{p}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <FieldSeparator />
          {/* ── 配置名称 ────────────────────────────────────────────────────── */}
          <Field>
            <FieldLabel>{t.agents.name}</FieldLabel>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My OpenAI Config"
            />
          </Field>
          {/* ── 模型名称 ────────────────────────────────────────────────────── */}
          <Field>
            <FieldLabel>{t.settings.modelSettings.model}</FieldLabel>
            <Input
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder={t.settings.modelSettings.modelPlaceholder}
            />
          </Field>
          {/* ── API 密钥 ────────────────────────────────────────────────────── */}
          <Field>
            <FieldLabel>{t.settings.modelSettings.apiKey}</FieldLabel>
            <Input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={t.settings.modelSettings.apiKeyPlaceholder}
            />
          </Field>
          {/* ── 基础 URL ────────────────────────────────────────────────────── */}
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
          <Button onClick={handleSave} disabled={!apiKey.trim() || !name.trim() || saving}>
            {saving ? <Loader2Icon className="size-4 animate-spin" /> : null}
            {t.settings.modelSettings.save}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}