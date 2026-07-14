import { useState } from "react";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Separator } from "@/components/ui/separator";
import { Palette, BarChart3, Loader2, FileText, Type, Layers, Activity } from "lucide-react";

interface StyleProfile {
  sourceName: string;
  avgSentenceLength: number;
  sentenceLengthStdDev: number;
  avgParagraphLength: number;
  vocabularyDiversity: number;
  topPatterns: string[];
  rhetoricalFeatures: string[];
  analyzedAt?: string;
}

export function StyleSettings() {
  const { t } = useI18n();
  const [text, setText] = useState("");
  const [sourceName, setSourceName] = useState("");
  const [profile, setProfile] = useState<StyleProfile | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleAnalyze = async () => {
    if (!text.trim()) return;

    setLoading(true);
    setError(null);
    setProfile(null);

    try {
      const result = analyzeStyle(text, sourceName || "sample");
      setProfile(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Analysis failed");
    } finally {
      setLoading(false);
    }
  };

  const handleClear = () => {
    setText("");
    setSourceName("");
    setProfile(null);
    setError(null);
  };

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <Palette />
            {t.settings.stylesTitle}
          </PageTitle>
          <PageDescription>{t.settings.stylesDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      <div className="flex flex-col gap-6">
        {/* 输入区域 */}
        <Card className="p-6">
          <div className="flex flex-col gap-6">
            <div className="flex items-center gap-3">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
                <FileText className="size-4 text-[var(--text-brand)]" />
              </div>
              <div>
                <h3 className="text-sm font-semibold">{t.settings.stylesInputTitle}</h3>
                <p className="text-xs text-[var(--text-tertiary)]">{t.settings.stylesTextSample}</p>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div className="md:col-span-1">
                <label className="trae-eyebrow mb-2 block">
                  {t.settings.stylesSourceName}
                </label>
                <Input
                  value={sourceName}
                  onChange={(e) => setSourceName(e.target.value)}
                  placeholder={t.settings.stylesSourcePlaceholder}
                />
              </div>
              <div className="md:col-span-3">
                <label className="trae-eyebrow mb-2 block">
                  {t.settings.stylesTextSample}
                </label>
                <Textarea
                  value={text}
                  onChange={(e) => setText(e.target.value)}
                  placeholder={t.settings.stylesTextPlaceholder}
                  rows={8}
                  className="font-mono text-sm"
                />
                <div className="mt-2 flex items-center justify-between text-xs text-[var(--text-tertiary)]">
                  <span>{t.settings.stylesCharCount}: <span className="trae-num">{text.length.toLocaleString()}</span></span>
                </div>
              </div>
            </div>

            <div className="flex gap-3">
              <Button onClick={handleAnalyze} disabled={!text.trim() || loading}>
                {loading ? (
                  <Loader2 className="size-4 animate-spin" data-icon="inline-start" />
                ) : (
                  <BarChart3 data-icon="inline-start" />
                )}
                {loading ? t.settings.stylesAnalyzing : t.settings.stylesAnalyze}
              </Button>
              <Button variant="outline" onClick={handleClear}>
                {t.settings.stylesClear}
              </Button>
            </div>

            {error && (
              <div className="text-sm text-[var(--status-error-default)] bg-[var(--status-error-surface-l1)] rounded-[var(--radius-4)] p-3">
                {error}
              </div>
            )}
          </div>
        </Card>

        {/* 分析结果 */}
        {profile ? (
          <div className="flex flex-col gap-6">
            {/* 统计卡片 */}
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
              <Card className="p-4">
                <div className="flex items-start gap-3">
                  <div className="flex size-10 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
                    <Type className="size-5 text-[var(--text-brand)]" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="trae-eyebrow">{t.settings.stylesAvgSentence}</p>
                    <p className="trae-stat-value mt-1.5">{profile.avgSentenceLength.toFixed(1)}</p>
                    <p className="text-xs text-[var(--text-tertiary)] mt-0.5">{t.settings.stylesChars}</p>
                  </div>
                </div>
              </Card>

              <Card className="p-4">
                <div className="flex items-start gap-3">
                  <div className="flex size-10 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
                    <Activity className="size-5 text-[var(--text-brand)]" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="trae-eyebrow">{t.settings.stylesVocabDiversity}</p>
                    <p className="trae-stat-value mt-1.5">{(profile.vocabularyDiversity * 100).toFixed(0)}%</p>
                    <Progress value={profile.vocabularyDiversity * 100} className="h-1.5 mt-2" />
                  </div>
                </div>
              </Card>

              <Card className="p-4">
                <div className="flex items-start gap-3">
                  <div className="flex size-10 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
                    <Layers className="size-5 text-[var(--text-brand)]" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="trae-eyebrow">{t.settings.stylesAvgParagraph}</p>
                    <p className="trae-stat-value mt-1.5">{profile.avgParagraphLength.toFixed(0)}</p>
                    <p className="text-xs text-[var(--text-tertiary)] mt-0.5">{t.settings.stylesChars}</p>
                  </div>
                </div>
              </Card>

              <Card className="p-4">
                <div className="flex items-start gap-3">
                  <div className="flex size-10 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
                    <BarChart3 className="size-5 text-[var(--text-brand)]" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="trae-eyebrow">{t.settings.stylesSentenceStdDev}</p>
                    <p className="trae-stat-value mt-1.5">{profile.sentenceLengthStdDev.toFixed(1)}</p>
                    <p className="text-xs text-[var(--text-tertiary)] mt-0.5">{t.settings.stylesChars}</p>
                  </div>
                </div>
              </Card>
            </div>

            {/* 模式和修辞特征 */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              {profile.topPatterns.length > 0 && (
                <Card className="p-5">
                  <h4 className="trae-card-eyebrow mb-3">{t.settings.stylesTopPatterns}</h4>
                  <div className="flex flex-wrap gap-2">
                    {profile.topPatterns.map((p) => (
                      <Badge key={p} variant="secondary" className="text-xs">
                        {p}
                      </Badge>
                    ))}
                  </div>
                </Card>
              )}

              {profile.rhetoricalFeatures.length > 0 && (
                <Card className="p-5">
                  <h4 className="trae-card-eyebrow mb-3">{t.settings.stylesRhetoricalFeatures}</h4>
                  <div className="flex flex-wrap gap-2">
                    {profile.rhetoricalFeatures.map((f) => (
                      <Badge key={f} className="text-xs bg-[var(--bg-brand-popup)] text-[var(--text-brand)] border-[var(--border-brand)]">
                        {f}
                      </Badge>
                    ))}
                  </div>
                </Card>
              )}
            </div>

            {/* 来源信息 */}
            <Separator />
            <div className="flex items-center gap-3 text-sm text-[var(--text-tertiary)]">
              <FileText className="size-4" />
              <span>{t.settings.stylesSource}: <span className="text-[var(--text-secondary)]">{profile.sourceName}</span></span>
              {profile.analyzedAt && (
                <span className="text-xs">({profile.analyzedAt})</span>
              )}
            </div>
          </div>
        ) : (
          <Card className="flex flex-col items-center justify-center py-16">
            <Palette className="size-12 text-[var(--text-tertiary)] mb-3 opacity-50" />
            <p className="text-sm text-[var(--text-tertiary)]">{t.settings.stylesEmptyHint}</p>
          </Card>
        )}
      </div>
    </PageContainer>
  );
}

function analyzeStyle(text: string, sourceName: string): StyleProfile {
  const sentences = text.split(/[。！？\n]+/).filter((s) => s.trim().length > 0);
  const paragraphs = text.split(/\n\n+/).filter((p) => p.trim().length > 0);
  const words = text.split(/\s+/).filter((w) => w.trim().length > 0);
  const uniqueWords = new Set(words.map((w) => w.toLowerCase()));

  const sentenceLengths = sentences.map((s) => s.length);
  const avgSentenceLength =
    sentenceLengths.length > 0
      ? sentenceLengths.reduce((a, b) => a + b, 0) / sentenceLengths.length
      : 0;

  const variance =
    sentenceLengths.length > 0
      ? sentenceLengths.reduce((acc, len) => acc + Math.pow(len - avgSentenceLength, 2), 0) /
        sentenceLengths.length
      : 0;
  const sentenceLengthStdDev = Math.sqrt(variance);

  const avgParagraphLength =
    paragraphs.length > 0
      ? paragraphs.reduce((a, p) => a + p.length, 0) / paragraphs.length
      : 0;

  const vocabularyDiversity = words.length > 0 ? uniqueWords.size / words.length : 0;

  const topPatterns = extractPatterns(text);
  const rhetoricalFeatures = extractRhetoricalFeatures(text);

  return {
    sourceName,
    avgSentenceLength,
    sentenceLengthStdDev,
    avgParagraphLength,
    vocabularyDiversity,
    topPatterns,
    rhetoricalFeatures,
    analyzedAt: new Date().toLocaleString(),
  };
}

function extractPatterns(text: string): string[] {
  const patterns: string[] = [];
  const regexes = [
    /[""「」『』]/g,
    /[！？]。/g,
    /……/g,
    /——/g,
    /\d+[%％]/g,
  ];
  const labels = ["引号", "叹问句", "省略号", "破折号", "数字"];

  regexes.forEach((regex, i) => {
    const matches = text.match(regex);
    if (matches && matches.length > 2) {
      patterns.push(labels[i]!);
    }
  });

  return patterns;
}

function extractRhetoricalFeatures(text: string): string[] {
  const features: string[] = [];

  if (text.includes("像") && text.split("像").length > 3) {
    features.push("明喻");
  }
  if (/[！？]{2,}/.test(text)) {
    features.push("强烈情感");
  }
  if (/[。]{2,}/.test(text)) {
    features.push("节奏变化");
  }
  if (/「[^」]+」/.test(text)) {
    features.push("对话");
  }
  if (/（[^）]+）/.test(text)) {
    features.push("括号注释");
  }

  return features;
}