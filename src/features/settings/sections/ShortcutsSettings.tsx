/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ShortcutsSettings - 快捷键设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useMemo, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { SearchIcon, RotateCcwIcon, Trash2Icon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { useShortcuts } from "@/features/settings/hooks";
import {
  SHORTCUTS,
  SHORTCUT_GROUPS,
  getBindingTokens,
  type KeyBinding,
  type Shortcut,
  type ShortcutId,
} from "@/lib/shortcuts";

export function ShortcutsSettings() {
  const { t } = useI18n();
  const shortcuts = useShortcuts();
  const [query, setQuery] = useState("");
  const [recordingId, setRecordingId] = useState<ShortcutId | null>(null);
  const [resetAllOpen, setResetAllOpen] = useState(false);

  const filtered = useMemo(() => {
    if (!query.trim()) return SHORTCUTS;
    const q = query.toLowerCase();
    return SHORTCUTS.filter((s) => {
      const label = t.shortcuts.items[s.id as keyof typeof t.shortcuts.items] ?? s.id;
      const group = t.shortcuts.groups[s.group];
      return label.toLowerCase().includes(q) || group.toLowerCase().includes(q);
    });
  }, [query, t]);

  const onRecord = (id: ShortcutId, binding: KeyBinding) => {
    void shortcuts.record(id, binding);
    setRecordingId(null);
  };

  const onClear = (id: ShortcutId) => {
    void shortcuts.clear(id);
  };

  const onReset = (id: ShortcutId) => {
    void shortcuts.reset(id);
  };

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.shortcuts}</PageTitle>
          <PageDescription>{t.settings.shortcutsDesc}</PageDescription>
        </PageHeading>
        <Button
          variant="outline"
          size="sm"
          className="h-8 gap-1.5"
          onClick={() => setResetAllOpen(true)}
        >
          <RotateCcwIcon className="size-3.5" />
          {t.shortcuts.resetAll}
        </Button>
      </PageHeader>

      {/* 搜索框 */}
      <div className="relative">
        <SearchIcon className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t.shortcuts.searchPlaceholder}
          className="h-9 pl-9 text-sm"
        />
      </div>

      {/* 分组列表 */}
      <div className="flex flex-col gap-6">
        {SHORTCUT_GROUPS.map((group) => {
          const items = filtered.filter((s) => s.group === group);
          if (items.length === 0) return null;
          return (
            <div key={group} className="flex flex-col gap-2">
              <h3 className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
                {t.shortcuts.groups[group]}
              </h3>
              <Card className="py-0">
                <CardContent className="divide-y px-0">
                  {items.map((s) => (
                    <ShortcutRow
                      key={s.id}
                      shortcut={s}
                      isRecording={recordingId === s.id}
                      onStartRecording={() => setRecordingId(s.id)}
                      onStopRecording={() => setRecordingId(null)}
                      onRecord={(b) => onRecord(s.id, b)}
                      onClear={() => onClear(s.id)}
                      onReset={() => onReset(s.id)}
                      userBindings={shortcuts.overrides[s.id]}
                    />
                  ))}
                </CardContent>
              </Card>
            </div>
          );
        })}
      </div>

      {/* 提示 */}
      <p className="text-xs text-muted-foreground">{t.shortcuts.customizeHint}</p>

      {/* 重置全部确认 */}
      <Dialog open={resetAllOpen} onOpenChange={setResetAllOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{t.shortcuts.resetAllTitle}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">{t.shortcuts.resetAllDesc}</p>
          <DialogFooter>
            <Button variant="outline" onClick={() => setResetAllOpen(false)}>
              {t.common.cancel}
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                setResetAllOpen(false);
                void shortcuts.resetAll();
              }}
            >
              {t.shortcuts.resetAllConfirm}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}

/** 单行：label + 绑定显示/录制 + 重置/清除按钮 */
function ShortcutRow({
  shortcut,
  isRecording,
  onStartRecording,
  onStopRecording,
  onRecord,
  onClear,
  onReset,
  userBindings,
}: {
  shortcut: Shortcut;
  isRecording: boolean;
  onStartRecording: () => void;
  onStopRecording: () => void;
  onRecord: (b: KeyBinding) => void;
  onClear: () => void;
  onReset: () => void;
  userBindings?: KeyBinding[];
}) {
  const { t } = useI18n();
  const label = t.shortcuts.items[shortcut.id as keyof typeof t.shortcuts.items] ?? shortcut.id;

  // 用户覆盖存在（含空数组）→ 用覆盖；否则用默认绑定
  const isModified = userBindings !== undefined;
  const bindings = isModified ? userBindings : shortcut.defaultBindings;
  const hasBindings = bindings && bindings.length > 0;

  return (
    <div className="group flex items-center justify-between gap-4 px-4 py-2.5 transition-colors hover:bg-muted/30">
      <span className="text-sm">{label}</span>
      <div className="flex items-center gap-2">
        {isRecording ? (
          <Recorder onRecord={onRecord} onCancel={onStopRecording} />
        ) : (
          <>
            <button
              type="button"
              onClick={onStartRecording}
              className="flex min-w-[100px] cursor-pointer items-center justify-end gap-1"
            >
              {hasBindings ? (
                getBindingTokens(bindings[0]).map((tok, i) => (
                  <Badge
                    key={i}
                    variant="outline"
                    className="gap-0.5 font-mono text-[11px] font-normal transition-colors group-hover:bg-accent group-hover:text-accent-foreground"
                  >
                    <span className="px-0.5">{tok}</span>
                  </Badge>
                ))
              ) : (
                <span className="text-[11px] text-muted-foreground italic">
                  {t.shortcuts.unassigned}
                </span>
              )}
            </button>
            <div className="flex items-center gap-1">
              {isModified && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="text-muted-foreground hover:text-foreground"
                  onClick={onReset}
                  title={t.shortcuts.resetToDefault}
                >
                  <RotateCcwIcon className="size-3.5" />
                </Button>
              )}
              <Button
                variant="ghost"
                size="icon-sm"
                className="text-muted-foreground hover:text-destructive opacity-0 transition-opacity group-hover:opacity-100"
                onClick={onClear}
                title={t.shortcuts.clearShortcut}
              >
                <Trash2Icon className="size-3.5" />
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

/** 录制器：监听 keydown，要求至少一个主修饰键（Ctrl/Alt/Meta），Esc 取消 */
function Recorder({
  onRecord,
  onCancel,
}: {
  onRecord: (b: KeyBinding) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();

  useEffect(() => {
    const onDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Esc 取消
      if (e.key === "Escape") {
        onCancel();
        return;
      }

      // 单独按下修饰键：不触发录制，等待后续主键
      const isMod = ["Control", "Shift", "Alt", "Meta"].includes(e.key);
      if (isMod) return;

      // 要求至少一个主修饰键（Ctrl/Alt/Meta）；拒绝 Shift-only 字符键
      // （避免 Shift+2 这种会被系统解释为输入字符的组合）
      const hasPrimaryModifier = e.ctrlKey || e.altKey || e.metaKey;
      const isCharacterKey = e.key.length === 1;
      if (!hasPrimaryModifier && (!e.shiftKey || isCharacterKey)) {
        return;
      }

      onRecord({
        key: e.key,
        ctrl: e.ctrlKey,
        shift: e.shiftKey,
        alt: e.altKey,
        meta: e.metaKey,
      });
    };

    window.addEventListener("keydown", onDown, { capture: true });
    return () => window.removeEventListener("keydown", onDown, { capture: true });
  }, [onRecord, onCancel]);

  return (
    <div className="flex items-center gap-2 rounded bg-accent/50 px-2 py-1 text-[11px] ring-1 ring-accent">
      <span className="animate-pulse font-medium">{t.shortcuts.recording}</span>
      <span className="text-muted-foreground">{t.shortcuts.recordingHint}</span>
    </div>
  );
}
