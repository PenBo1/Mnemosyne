/**
 * ═══════════════════════════════════════════════════════════════════════════
 * AgentIdentityBlock - Agent 身份文件编辑区块
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useI18n } from "@/locales/i18n";
import { useAsyncAction } from "@/hooks/useAsyncAction";
import { SettingsSection } from "@/features/settings/components/settings-section";
import {
  IDENTITY_FILES,
  loadIdentityFile,
  saveIdentityFile,
  type IdentityFileName,
} from "@/features/settings/services/agent-identity";

// ── 常量配置 ────────────────────────────────────────────────────────────────

/** 当前支持的角色列表 */
const ROLES = ["main"] as const;
type AgentRole = (typeof ROLES)[number];

/** 三种身份文件的标签 key 后缀 */
const FILE_LABEL_KEYS: Record<IdentityFileName, string> = {
  "SOUL.md": "soulMd",
  "CONTEXT.md": "contextMd",
  "MEMORY.md": "memoryMd",
};

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 创建空内容对象
 */
function emptyContents(): Record<IdentityFileName, string> {
  return { "SOUL.md": "", "CONTEXT.md": "", "MEMORY.md": "" };
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * Agent 身份文件编辑区块，用于编辑 SOUL.md、CONTEXT.md、MEMORY.md 文件
 */
export function AgentIdentityBlock() {
  const { t } = useI18n();
  const { loading: saving, run } = useAsyncAction();
  const [role, setRole] = useState<AgentRole>("main");
  const [activeFile, setActiveFile] = useState<IdentityFileName>("SOUL.md");
  const [contents, setContents] = useState<Record<IdentityFileName, string>>(emptyContents);
  const [originals, setOriginals] = useState<Record<IdentityFileName, string>>(emptyContents);
  const [loading, setLoading] = useState(true);
  const ai = t.settings.agentIdentity;

  // ── 数据加载 ──────────────────────────────────────────────────────────────

  const loadAll = useCallback(async (r: string) => {
    setLoading(true);
    try {
      const entries = await Promise.all(
        IDENTITY_FILES.map(async (f) => [f, await loadIdentityFile(r, f)] as const),
      );
      const next = emptyContents();
      for (const [f, c] of entries) next[f] = c;
      setContents(next);
      setOriginals(next);
    } catch (err) {
      const msg = err instanceof Error ? err.message : ai.loadError;
      toast.error(msg);
    } finally {
      setLoading(false);
    }
  }, [ai.loadError]);

  useEffect(() => {
    void loadAll(role);
  }, [role, loadAll]);

  // ── 计算属性 ──────────────────────────────────────────────────────────────

  const dirty = contents[activeFile] !== originals[activeFile];

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  /**
   * 保存文件
   */
  const handleSave = useCallback(async () => {
    await run(
      () => saveIdentityFile(role, activeFile, contents[activeFile]),
      {
        successToast: t.settings.agentIdentity.saved,
        errorToast: t.settings.agentIdentity.saveError,
      },
    );
    setOriginals((prev) => ({ ...prev, [activeFile]: contents[activeFile] }));
  }, [run, role, activeFile, contents, ai.saved, ai.saveError]);

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <SettingsSection title={ai.title} description={ai.description}>
      <div className="flex flex-col gap-3 px-4 py-3">
        {/* ── 角色选择器和保存按钮 ──────────────────────────────────────────── */}
        <div className="flex items-center justify-between gap-2">
          <div className="flex flex-col gap-0.5">
            <span className="text-sm font-medium">{ai.selectRole}</span>
            <span className="text-xs text-muted-foreground">{ai.selectRoleHint}</span>
          </div>
          <div className="flex items-center gap-2">
            <Select value={role} onValueChange={(v) => setRole(v as AgentRole)}>
              <SelectTrigger className="w-32">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ROLES.map((r) => (
                  <SelectItem key={r} value={r}>{r}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Button
              size="sm"
              className="h-8 gap-1.5 px-2 text-xs"
              onClick={handleSave}
              disabled={!dirty || saving || loading}
            >
              {saving ? t.common.saving : t.common.save}
            </Button>
          </div>
        </div>

        {/* ── 文件编辑器 ────────────────────────────────────────────────────── */}
        <Tabs
          value={activeFile}
          onValueChange={(v) => setActiveFile(v as IdentityFileName)}
        >
          <TabsList>
            {IDENTITY_FILES.map((f) => (
              <TabsTrigger key={f} value={f}>
                {ai[FILE_LABEL_KEYS[f] as keyof typeof ai] as string}
              </TabsTrigger>
            ))}
          </TabsList>
          {IDENTITY_FILES.map((f) => (
            <TabsContent key={f} value={f}>
              <Textarea
                value={contents[f]}
                onChange={(e) =>
                  setContents((prev) => ({ ...prev, [f]: e.target.value }))
                }
                placeholder={ai.editorPlaceholder}
                disabled={loading}
                className="min-h-[280px] resize-y font-mono text-xs"
              />
              {contents[f] !== originals[f] && (
                <p className="mt-1 text-[11px] text-amber-600">{ai.unsaved}</p>
              )}
            </TabsContent>
          ))}
        </Tabs>
      </div>
    </SettingsSection>
  );
}