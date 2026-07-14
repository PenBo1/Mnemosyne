// 用户画像设置页 —— 查看/编辑当前用户画像（UserProfile）。
//
// 后端 IPC:
// - user_get_profile: 读取画像
// - user_update_profile: 整体覆盖更新画像
//
// 字段命名注意：后端 UserProfile 结构未启用 serde rename_all = "camelCase"，
// 故前端类型与 IPC 负载均使用 snake_case 字段名（见 types/user-profile.ts）。

import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useI18n } from "@/locales/i18n";
import { useAsyncAction } from "@/hooks/useAsyncAction";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { SettingsSection, SettingsRow } from "@/features/settings/components/settings-section";
import { LoadingState } from "@/components/shared/state";
import { getUserProfile, updateUserProfile } from "@/features/settings/services/user-profile";
import {
  READER_TYPE_VARIANTS,
  customReaderTypeValue,
  isCustomReaderType,
  readerTypeKey,
  type ReaderType,
  type UserProfile,
} from "@/features/settings/types/user-profile";

/** 将字符串数组转为换行分隔的文本（用于 textarea 编辑） */
function linesToText(arr: string[]): string {
  return arr.join("\n");
}

/** 将换行分隔文本转为字符串数组（过滤空行） */
function textToLines(text: string): string[] {
  return text
    .split("\n")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

/** 将字符串数组转为逗号分隔文本（用于 input 编辑） */
function tagsToText(arr: string[]): string {
  return arr.join(", ");
}

/** 将逗号分隔文本转为字符串数组（过滤空项） */
function textToTags(text: string): string[] {
  return text
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

export function UserProfileSettings() {
  const { t } = useI18n();
  const { loading, run } = useAsyncAction();
  const [draft, setDraft] = useState<UserProfile | null>(null);
  const [initial, setInitial] = useState<UserProfile | null>(null);

  const loadData = useCallback(async () => {
    const profile = await run(() => getUserProfile(), {
      errorToast: t.settings.userProfile.loadError,
    });
    if (profile) {
      setDraft(profile);
      setInitial(profile);
    }
  }, [run, t.settings.userProfile.loadError]);

  useEffect(() => {
    void loadData();
    // 仅在挂载时加载一次
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const dirty = !!draft && !!initial && JSON.stringify(draft) !== JSON.stringify(initial);

  const handleSave = useCallback(async () => {
    if (!draft) return;
    const saved = await run(() => updateUserProfile(draft), {
      successToast: t.settings.userProfile.saved,
      errorToast: t.settings.userProfile.saveError,
    });
    if (saved) {
      setDraft(saved);
      setInitial(saved);
    }
  }, [draft, run, t.settings.userProfile.saved, t.settings.userProfile.saveError]);

  const handleReset = useCallback(() => {
    if (initial) setDraft(initial);
  }, [initial]);

  // 局部更新工具
  const patch = useCallback(<K extends keyof UserProfile>(key: K, value: UserProfile[K]) => {
    setDraft((prev) => (prev ? { ...prev, [key]: value } : prev));
  }, []);

  if (!draft) {
    return (
      <PageContainer scrollable={false}>
        <LoadingState label={t.common.loading} />
      </PageContainer>
    );
  }

  const up = t.settings.userProfile;
  const rtKey = readerTypeKey(draft.reader_type);
  const rtCustom = customReaderTypeValue(draft.reader_type);
  const wcp = draft.word_count_preference;

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{up.title}</PageTitle>
          <PageDescription>{up.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button
            variant="outline"
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            onClick={handleReset}
            disabled={!dirty || loading}
          >
            {up.reset}
          </Button>
          <Button
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            onClick={handleSave}
            disabled={!dirty || loading}
          >
            {loading ? t.common.saving : t.common.save}
          </Button>
        </PageActions>
      </PageHeader>

      {/* 基本信息 */}
      <SettingsSection title={up.sectionBasic}>
        <SettingsRow label={up.name} description={up.nameHint}>
          <Input
            value={draft.name}
            onChange={(e) => patch("name", e.target.value)}
            className="h-8 w-48 text-xs"
            placeholder={up.namePlaceholder}
          />
        </SettingsRow>
        <SettingsRow label={up.language} description={up.languageHint}>
          <Select
            value={draft.language}
            onValueChange={(v) => patch("language", v)}
          >
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="auto">{up.languageAuto}</SelectItem>
              <SelectItem value="zh">中文</SelectItem>
              <SelectItem value="en">English</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>
      </SettingsSection>

      {/* 写作风格 */}
      <SettingsSection title={up.sectionStyle} description={up.styleHint}>
        <SettingsRow label={up.styleFormality} description={up.styleFormalityHint}>
          <Select
            value={draft.style.formality}
            onValueChange={(v) => patch("style", { ...draft.style, formality: v })}
          >
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="standard">{up.styleFormalityStandard}</SelectItem>
              <SelectItem value="formal">{up.styleFormalityFormal}</SelectItem>
              <SelectItem value="casual">{up.styleFormalityCasual}</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>
        <SettingsRow label={up.stylePacing} description={up.stylePacingHint}>
          <Select
            value={draft.style.pacing}
            onValueChange={(v) => patch("style", { ...draft.style, pacing: v })}
          >
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="fast">{up.stylePacingFast}</SelectItem>
              <SelectItem value="moderate">{up.stylePacingModerate}</SelectItem>
              <SelectItem value="slow">{up.stylePacingSlow}</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>
        <SettingsRow label={up.styleDensity} description={up.styleDensityHint}>
          <Select
            value={draft.style.description_density}
            onValueChange={(v) =>
              patch("style", { ...draft.style, description_density: v })
            }
          >
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="sparse">{up.styleDensitySparse}</SelectItem>
              <SelectItem value="moderate">{up.styleDensityModerate}</SelectItem>
              <SelectItem value="dense">{up.styleDensityDense}</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>
        <SettingsRow label={up.styleDialogue} description={up.styleDialogueHint}>
          <Select
            value={draft.style.dialogue_style}
            onValueChange={(v) =>
              patch("style", { ...draft.style, dialogue_style: v })
            }
          >
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="natural">{up.styleDialogueNatural}</SelectItem>
              <SelectItem value="stylized">{up.styleDialogueStylized}</SelectItem>
              <SelectItem value="minimal">{up.styleDialogueMinimal}</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>
      </SettingsSection>

      {/* 读者类型 */}
      <SettingsSection title={up.readerType} description={up.readerTypeHint}>
        <SettingsRow label={up.readerType} description={up.readerTypeHint}>
          <Select
            value={rtKey}
            onValueChange={(v) => {
              const next: ReaderType =
                v === "Custom" ? { Custom: rtCustom } : (v as ReaderType);
              patch("reader_type", next);
            }}
          >
            <SelectTrigger className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {READER_TYPE_VARIANTS.map((v) => (
                <SelectItem key={v} value={v}>
                  {up[`readerType${v}` as keyof typeof up] as string}
                </SelectItem>
              ))}
              <SelectItem value="Custom">{up.readerTypeCustom}</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>
        {isCustomReaderType(draft.reader_type) && (
          <SettingsRow label={up.readerTypeCustomLabel} description={up.readerTypeCustomHint}>
            <Input
              value={rtCustom}
              onChange={(e) =>
                patch("reader_type", { Custom: e.target.value })
              }
              className="h-8 w-48 text-xs"
              placeholder={up.readerTypeCustomPlaceholder}
            />
          </SettingsRow>
        )}
      </SettingsSection>

      {/* 偏好题材与语气 */}
      <SettingsSection title={up.sectionPreferences} description={up.preferencesHint}>
        <SettingsRow label={up.genres} description={up.genresHint}>
          <Input
            value={tagsToText(draft.genres)}
            onChange={(e) => patch("genres", textToTags(e.target.value))}
            className="h-8 w-64 text-xs"
            placeholder={up.genresPlaceholder}
          />
        </SettingsRow>
        <SettingsRow label={up.tone} description={up.toneHint}>
          <Textarea
            value={draft.tone ?? ""}
            onChange={(e) => patch("tone", e.target.value || null)}
            className="min-h-[80px] resize-y text-xs"
            placeholder={up.tonePlaceholder}
          />
        </SettingsRow>
        <SettingsRow label={up.customInstructions} description={up.customInstructionsHint}>
          <Textarea
            value={linesToText(draft.custom_instructions)}
            onChange={(e) =>
              patch("custom_instructions", textToLines(e.target.value))
            }
            className="min-h-[100px] resize-y text-xs"
            placeholder={up.customInstructionsPlaceholder}
          />
        </SettingsRow>
      </SettingsSection>

      {/* 字数偏好 */}
      <SettingsSection title={up.sectionWordCount} description={up.wordCountHint}>
        <SettingsRow label={up.wordCountEnable} description={up.wordCountEnableHint}>
          <Switch
            checked={wcp !== null}
            onCheckedChange={(checked) => {
              patch("word_count_preference", checked ? { min_words: 2000, max_words: 5000, target_words: 3000 } : null);
            }}
          />
        </SettingsRow>
        {wcp !== null && (
          <>
            <SettingsRow label={up.wordCountMin} description={up.wordCountMinHint}>
              <Input
                type="number"
                min={0}
                value={wcp.min_words}
                onChange={(e) =>
                  patch("word_count_preference", {
                    ...wcp,
                    min_words: Math.max(0, Number(e.target.value) || 0),
                  })
                }
                className="h-8 w-32 text-xs"
              />
            </SettingsRow>
            <SettingsRow label={up.wordCountTarget} description={up.wordCountTargetHint}>
              <Input
                type="number"
                min={0}
                value={wcp.target_words}
                onChange={(e) =>
                  patch("word_count_preference", {
                    ...wcp,
                    target_words: Math.max(0, Number(e.target.value) || 0),
                  })
                }
                className="h-8 w-32 text-xs"
              />
            </SettingsRow>
            <SettingsRow label={up.wordCountMax} description={up.wordCountMaxHint}>
              <Input
                type="number"
                min={0}
                value={wcp.max_words}
                onChange={(e) =>
                  patch("word_count_preference", {
                    ...wcp,
                    max_words: Math.max(0, Number(e.target.value) || 0),
                  })
                }
                className="h-8 w-32 text-xs"
              />
            </SettingsRow>
          </>
        )}
      </SettingsSection>
    </PageContainer>
  );
}
