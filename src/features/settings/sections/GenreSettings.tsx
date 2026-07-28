/**
 * ═══════════════════════════════════════════════════════════════════════════
 * GenreSettings - 题材设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useEffect } from "react";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Button } from "@/components/ui/button";
import { ChevronDown, BookMarked, Zap, TrendingUp, Layers } from "lucide-react";
import { cn } from "@/lib/utils";
import type { GenreProfile } from "@/types/genre-profile";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface GenreListItem {
  id: string;
  nameKey: string;
  source: "builtin" | "project";
}

type GenreTranslations = {
  xianxia: string;
  xuanhuan: string;
  urban: string;
  litrpg: string;
  progression: string;
  cozy: string;
  cultivation: string;
  "dungeon-core": string;
  horror: string;
  isekai: string;
  romantasy: string;
  "sci-fi": string;
  "system-apocalypse": string;
  "tower-climber": string;
  other: string;
  chapterTypes: Record<string, string>;
  fatigueWords: Record<string, string>;
  satisfactionTypes: Record<string, string>;
  pacingRules: Record<string, string>;
};

// ── 常量配置 ────────────────────────────────────────────────────────────────

const BUILTIN_GENRES: GenreListItem[] = [
  { id: "xianxia", nameKey: "xianxia", source: "builtin" },
  { id: "xuanhuan", nameKey: "xuanhuan", source: "builtin" },
  { id: "urban", nameKey: "urban", source: "builtin" },
  { id: "litrpg", nameKey: "litrpg", source: "builtin" },
  { id: "progression", nameKey: "progression", source: "builtin" },
  { id: "cozy", nameKey: "cozy", source: "builtin" },
  { id: "cultivation", nameKey: "cultivation", source: "builtin" },
  { id: "dungeon-core", nameKey: "dungeon-core", source: "builtin" },
  { id: "horror", nameKey: "horror", source: "builtin" },
  { id: "isekai", nameKey: "isekai", source: "builtin" },
  { id: "romantasy", nameKey: "romantasy", source: "builtin" },
  { id: "sci-fi", nameKey: "sci-fi", source: "builtin" },
  { id: "system-apocalypse", nameKey: "system-apocalypse", source: "builtin" },
  { id: "tower-climber", nameKey: "tower-climber", source: "builtin" },
  { id: "other", nameKey: "other", source: "builtin" },
];

// ── 辅助函数 ────────────────────────────────────────────────────────────────

function getGenreData(t: { agentChat: { genreData: GenreTranslations } }): GenreTranslations {
  return t.agentChat.genreData;
}

function getDefaultChapterTypes(id: string, genreData: GenreTranslations): string[] {
  const ct = genreData.chapterTypes;
  const types: Record<string, string[]> = {
    xianxia: [ct.battle, ct.enlightenment, ct.setup, ct.transition, ct.payoff].filter(Boolean),
    xuanhuan: [ct.battle, ct.exploration, ct.upgrade, ct.transition].filter(Boolean),
    urban: [ct.daily, ct.business, ct.romance, ct.turning].filter(Boolean),
    litrpg: [ct.mission, ct.battle, ct.upgrade, ct.daily].filter(Boolean),
    progression: [ct.upgrade, ct.battle, ct.cultivation, ct.harvest].filter(Boolean),
    other: [ct.plot, ct.transition].filter(Boolean),
  };
  return types[id] ?? types.other;
}

function getDefaultFatigueWords(id: string, genreData: GenreTranslations): string[] {
  const fw = genreData.fatigueWords;
  const words: Record<string, string[]> = {
    xianxia: [fw.sneer, fw.ant, fw.gasp, fw.pupilShrink, fw.heavenlyDao, fw.greatDao, fw.karma, fw.fortune].filter(Boolean),
    xuanhuan: [fw.sneer, fw.ant, fw.gasp, fw.pupilShrink].filter(Boolean),
    urban: [fw.faintly, fw.slightly, fw.lightSmile, fw.cornerUp].filter(Boolean),
    other: [],
  };
  return words[id] ?? words.other;
}

function getDefaultPacingRule(id: string, genreData: GenreTranslations): string {
  return genreData.pacingRules[id] ?? genreData.pacingRules.other ?? "";
}

function getDefaultSatisfactionTypes(id: string, genreData: GenreTranslations): string[] {
  const st = genreData.satisfactionTypes;
  const types: Record<string, string[]> = {
    xianxia: [st.enlightenmentBreakthrough, st.combatCrush, st.treasureGain, st.identityReveal, st.tribulationPass, st.karmaResolve].filter(Boolean),
    xuanhuan: [st.upgradeBreakthrough, st.treasureObtain, st.powerCrush, st.opportunityGain].filter(Boolean),
    urban: [st.businessSuccess, st.relationshipProgress, st.identityChange, st.dilemmaBreak].filter(Boolean),
    progression: [st.upgradeBreakthrough, st.skillObtain, st.powerCrush, st.goalAchieve].filter(Boolean),
    other: [st.goalAchieve, st.dilemmaBreak].filter(Boolean),
  };
  return types[id] ?? types.other;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 题材设置页面，用于选择和管理小说题材配置
 */
export function GenreSettings() {
  const { t } = useI18n();
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [selectedGenres, setSelectedGenres] = useState<Set<string>>(new Set());
  const [genreProfiles, setGenreProfiles] = useState<Record<string, GenreProfile>>({});

  // ── 初始化 ────────────────────────────────────────────────────────────────

  useEffect(() => {
    const genreData = getGenreData(t as { agentChat: { genreData: GenreTranslations } });
    const loadedProfiles: Record<string, GenreProfile> = {};
    BUILTIN_GENRES.forEach((g) => {
      loadedProfiles[g.id] = {
        id: g.id,
        name: genreData[g.nameKey as keyof GenreTranslations] as string,
        language: "zh",
        chapterTypes: getDefaultChapterTypes(g.id, genreData),
        fatigueWords: getDefaultFatigueWords(g.id, genreData),
        numericalSystem: ["xianxia", "xuanhuan", "litrpg", "progression", "cultivation", "tower-climber"].includes(g.id),
        powerScaling: ["xianxia", "xuanhuan", "litrpg", "progression", "cultivation", "tower-climber", "system-apocalypse"].includes(g.id),
        eraResearch: ["urban", "sci-fi"].includes(g.id),
        pacingRule: getDefaultPacingRule(g.id, genreData),
        satisfactionTypes: getDefaultSatisfactionTypes(g.id, genreData),
        auditDimensions: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
      };
    });
    setGenreProfiles(loadedProfiles);
  }, [t]);

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  const toggleGenre = (id: string) => {
    setSelectedGenres((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const toggleExpand = (id: string) => {
    setExpandedId((prev) => (prev === id ? null : id));
  };

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <BookMarked />
            {t.settings.genresTitle}
          </PageTitle>
          <PageDescription>{t.settings.genresDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      <div className="flex flex-col gap-5">
        {/* ── 工具栏 ──────────────────────────────────────────────────────── */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="text-sm text-[var(--text-tertiary)]">
              {t.settings.genresSelected}:
            </span>
            <Badge variant="secondary" className="trae-num">
              {selectedGenres.size}
            </Badge>
          </div>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setSelectedGenres(new Set(BUILTIN_GENRES.map((g) => g.id)))}
            >
              {t.settings.genresSelectAll}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setSelectedGenres(new Set())}
            >
              {t.settings.genresClearAll}
            </Button>
          </div>
        </div>

        {/* ── 题材卡片网格 ────────────────────────────────────────────────── */}
        <ScrollArea className="flex-1 -mx-2 px-2">
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
            {BUILTIN_GENRES.map((genre) => {
              const profile = genreProfiles[genre.id];
              const isSelected = selectedGenres.has(genre.id);
              const isExpanded = expandedId === genre.id;

              return (
                <Card
                  key={genre.id}
                  className={cn(
                    "transition-all",
                    isSelected && "border-[var(--border-brand)] bg-[var(--bg-brand-popup)]"
                  )}
                >
                  <Collapsible open={isExpanded} onOpenChange={() => toggleExpand(genre.id)}>
                    {/* ── 卡片头部 ────────────────────────────────────────── */}
                    <div className="flex items-center justify-between p-4">
                      <div className="flex items-center gap-3 min-w-0">
                        <div className="flex size-9 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
                          <BookMarked className="size-4 text-[var(--text-brand)]" />
                        </div>
                        <div className="min-w-0">
                          <h3 className="text-sm font-semibold truncate">{profile?.name ?? genre.id}</h3>
                          <Badge variant="outline" className="text-[10px] mt-1">
                            {genre.source === "builtin" ? t.settings.genresBuiltin : t.settings.genresProject}
                          </Badge>
                        </div>
                      </div>
                      <Switch
                        checked={isSelected}
                        onCheckedChange={() => toggleGenre(genre.id)}
                      />
                    </div>

                    {/* ── 展开触发器 ──────────────────────────────────────── */}
                    <CollapsibleTrigger asChild>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="w-full justify-between rounded-none border-t border-[var(--border-neutral-l1)]"
                      >
                        <span className="text-xs text-[var(--text-tertiary)]">
                          {t.settings.genresShowDetails}
                        </span>
                        <ChevronDown
                          className={cn(
                            "size-4 transition-transform text-[var(--text-tertiary)]",
                            isExpanded && "rotate-180"
                          )}
                        />
                      </Button>
                    </CollapsibleTrigger>

                    {/* ── 展开内容 ────────────────────────────────────────── */}
                    <CollapsibleContent>
                      <CardContent className="pt-4 border-t border-[var(--border-neutral-l1)]">
                        {profile && (
                          <div className="flex flex-col gap-4 text-sm">
                            {/* ── 特性标签 ────────────────────────────────── */}
                            <div className="flex items-center gap-3">
                              <div className="flex items-center gap-2">
                                <Zap className="size-4 text-[var(--text-tertiary)]" />
                                <span className="text-[var(--text-tertiary)] text-xs">{t.settings.genresNumericalSystem}:</span>
                                <Badge variant={profile.numericalSystem ? "default" : "secondary"} className="text-xs">
                                  {profile.numericalSystem ? t.common.yes : t.common.no}
                                </Badge>
                              </div>
                              <div className="flex items-center gap-2">
                                <TrendingUp className="size-4 text-[var(--text-tertiary)]" />
                                <span className="text-[var(--text-tertiary)] text-xs">{t.settings.genresPowerScaling}:</span>
                                <Badge variant={profile.powerScaling ? "default" : "secondary"} className="text-xs">
                                  {profile.powerScaling ? t.common.yes : t.common.no}
                                </Badge>
                              </div>
                            </div>

                            {/* ── 章节类型 ──────────────────────────────── */}
                            {profile.chapterTypes.length > 0 && (
                              <div>
                                <div className="trae-eyebrow mb-2">{t.settings.genresChapterTypes}</div>
                                <div className="flex flex-wrap gap-1.5">
                                  {profile.chapterTypes.map((type) => (
                                    <Badge key={type} variant="outline" className="text-xs">
                                      {type}
                                    </Badge>
                                  ))}
                                </div>
                              </div>
                            )}

                            {/* ── 疲劳词 ────────────────────────────────── */}
                            {profile.fatigueWords.length > 0 && (
                              <div>
                                <div className="trae-eyebrow mb-2 flex items-center gap-2">
                                  {t.settings.genresFatigueWords}
                                  <Badge variant="secondary" className="text-xs trae-num">
                                    {profile.fatigueWords.length}
                                  </Badge>
                                </div>
                                <ScrollArea className="h-14">
                                  <div className="flex flex-wrap gap-1.5">
                                    {profile.fatigueWords.map((word) => (
                                      <Badge key={word} variant="secondary" className="text-xs">
                                        {word}
                                      </Badge>
                                    ))}
                                  </div>
                                </ScrollArea>
                              </div>
                            )}

                            {/* ── 满足感类型 ──────────────────────────── */}
                            {profile.satisfactionTypes.length > 0 && (
                              <div>
                                <div className="trae-eyebrow mb-2">{t.settings.genresSatisfactionTypes}</div>
                                <div className="flex flex-wrap gap-1.5">
                                  {profile.satisfactionTypes.map((type) => (
                                    <Badge key={type} className="text-xs bg-[var(--bg-brand-popup)] text-[var(--text-brand)] border-[var(--border-brand)]">
                                      {type}
                                    </Badge>
                                  ))}
                                </div>
                              </div>
                            )}

                            {/* ── 节奏规则 ──────────────────────────── */}
                            {profile.pacingRule && (
                              <div>
                                <div className="trae-eyebrow mb-2 flex items-center gap-2">
                                  <Layers className="size-3" />
                                  {t.settings.genresPacingRule}
                                </div>
                                <p className="text-xs text-[var(--text-secondary)] bg-[var(--bg-overlay-l2)] rounded-[var(--radius-4)] p-3">
                                  {profile.pacingRule}
                                </p>
                              </div>
                            )}
                          </div>
                        )}
                      </CardContent>
                    </CollapsibleContent>
                  </Collapsible>
                </Card>
              );
            })}
          </div>
        </ScrollArea>
      </div>
    </PageContainer>
  );
}