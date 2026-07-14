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

interface GenreListItem {
  id: string;
  name: string;
  source: "builtin" | "project";
}

const BUILTIN_GENRES: GenreListItem[] = [
  { id: "xianxia", name: "仙侠", source: "builtin" },
  { id: "xuanhuan", name: "玄幻", source: "builtin" },
  { id: "urban", name: "都市", source: "builtin" },
  { id: "litrpg", name: "LitRPG", source: "builtin" },
  { id: "progression", name: "升级流", source: "builtin" },
  { id: "cozy", name: "轻松日常", source: "builtin" },
  { id: "cultivation", name: "修炼流", source: "builtin" },
  { id: "dungeon-core", name: "地牢核心", source: "builtin" },
  { id: "horror", name: "恐怖", source: "builtin" },
  { id: "isekai", name: "异世界", source: "builtin" },
  { id: "romantasy", name: "浪漫奇幻", source: "builtin" },
  { id: "sci-fi", name: "科幻", source: "builtin" },
  { id: "system-apocalypse", name: "系统末世", source: "builtin" },
  { id: "tower-climber", name: "爬塔流", source: "builtin" },
  { id: "other", name: "通用", source: "builtin" },
];

export function GenreSettings() {
  const { t } = useI18n();
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [selectedGenres, setSelectedGenres] = useState<Set<string>>(new Set());
  const [genreProfiles, setGenreProfiles] = useState<Record<string, GenreProfile>>({});

  useEffect(() => {
    const loadedProfiles: Record<string, GenreProfile> = {};
    BUILTIN_GENRES.forEach((g) => {
      loadedProfiles[g.id] = {
        id: g.id,
        name: g.name,
        language: "zh",
        chapterTypes: getDefaultChapterTypes(g.id),
        fatigueWords: getDefaultFatigueWords(g.id),
        numericalSystem: ["xianxia", "xuanhuan", "litrpg", "progression", "cultivation", "tower-climber"].includes(g.id),
        powerScaling: ["xianxia", "xuanhuan", "litrpg", "progression", "cultivation", "tower-climber", "system-apocalypse"].includes(g.id),
        eraResearch: ["urban", "sci-fi"].includes(g.id),
        pacingRule: getDefaultPacingRule(g.id),
        satisfactionTypes: getDefaultSatisfactionTypes(g.id),
        auditDimensions: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
      };
    });
    setGenreProfiles(loadedProfiles);
  }, []);

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

  return (
    <PageContainer scrollable={false}>
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
        {/* 工具栏 */}
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

        {/* 题材卡片网格 */}
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
                    {/* 卡片头部 */}
                    <div className="flex items-center justify-between p-4">
                      <div className="flex items-center gap-3 min-w-0">
                        <div className="flex size-9 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
                          <BookMarked className="size-4 text-[var(--text-brand)]" />
                        </div>
                        <div className="min-w-0">
                          <h3 className="text-sm font-semibold truncate">{genre.name}</h3>
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

                    {/* 展开触发器 */}
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

                    {/* 展开内容 */}
                    <CollapsibleContent>
                      <CardContent className="pt-4 border-t border-[var(--border-neutral-l1)]">
                        {profile && (
                          <div className="flex flex-col gap-4 text-sm">
                            {/* 特性标签 */}
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

                            {/* 章节类型 */}
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

                            {/* 疲劳词 */}
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

                            {/* 满足感类型 */}
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

                            {/* 节奏规则 */}
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

function getDefaultChapterTypes(id: string): string[] {
  const types: Record<string, string[]> = {
    xianxia: ["战斗章", "悟道章", "布局章", "过渡章", "回收章"],
    xuanhuan: ["战斗章", "探险章", "升级章", "过渡章"],
    urban: ["日常章", "商战章", "感情章", "转折章"],
    litrpg: ["任务章", "战斗章", "升级章", "日常章"],
    progression: ["升级章", "战斗章", "修炼章", "收获章"],
    other: ["情节章", "过渡章"],
  };
  return types[id] ?? types.other;
}

function getDefaultFatigueWords(id: string): string[] {
  const words: Record<string, string[]> = {
    xianxia: ["冷笑", "蝼蚁", "倒吸凉气", "瞳孔骤缩", "天道", "大道", "因果", "气运"],
    xuanhuan: ["冷笑", "蝼蚁", "倒吸凉气", "瞳孔骤缩"],
    urban: ["淡淡", "微微", "轻笑", "嘴角上扬"],
    other: [],
  };
  return words[id] ?? words.other;
}

function getDefaultPacingRule(id: string): string {
  const rules: Record<string, string> = {
    xianxia: "修炼/悟道与战斗交替，每3-5章一次小突破或关键收获",
    xuanhuan: "探险与升级交替，每章有明确目标",
    urban: "日常与转折交替，感情线稳步推进",
    progression: "升级节奏紧凑，每章有可见进展",
    other: "情节张弛有度，每章有明确目的",
  };
  return rules[id] ?? rules.other;
}

function getDefaultSatisfactionTypes(id: string): string[] {
  const types: Record<string, string[]> = {
    xianxia: ["悟道突破", "斗法碾压", "法宝收获", "身份揭示", "天劫渡过", "因果了结"],
    xuanhuan: ["升级突破", "宝物收获", "实力碾压", "机缘获得"],
    urban: ["商业成功", "感情进展", "身份转变", "困境突破"],
    progression: ["升级成功", "技能获得", "实力碾压", "目标达成"],
    other: ["目标达成", "困境突破"],
  };
  return types[id] ?? types.other;
}