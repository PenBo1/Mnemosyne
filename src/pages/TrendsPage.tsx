import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Radar as RadarIcon,
  ScanSearchIcon,
  Trash2Icon,
  MoreVerticalIcon,
  ClockIcon,
  TrendingUpIcon,
  BookOpenIcon,
} from "lucide-react";
import { PageContainer, PageHeader, PageHeading, PageTitle, PageDescription, PageActions } from "@/components/shared/page-layout";
import { EmptyState } from "@/components/shared/state";
import { useI18n } from "@/shared/i18n";
import { useRadar } from "@/features/radar/hooks/useRadar";
import type { RadarRecommendation } from "@/shared/types";

function ConfidenceBadge({ confidence }: { confidence: number }) {
  const pct = Math.round(confidence * 100);
  let color = "bg-muted text-muted-foreground";
  if (confidence >= 0.7) color = "bg-success/10 text-success";
  else if (confidence >= 0.4) color = "bg-warning/10 text-warning";
  return (
    <Badge variant="outline" className={`${color} border-0 font-mono text-xs`}>
      {pct}%
    </Badge>
  );
}

function RecommendationCard({ rec }: { rec: RadarRecommendation }) {
  return (
    <div className="rounded-[var(--radius-6)] border bg-card p-5 flex flex-col gap-3 transition-shadow">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {rec.platform}
          </span>
          <span className="text-border">·</span>
          <span className="text-xs font-semibold text-primary">{rec.genre}</span>
        </div>
        <ConfidenceBadge confidence={rec.confidence} />
      </div>
      <p className="text-sm font-semibold leading-snug">{rec.concept}</p>
      <p className="text-xs text-muted-foreground leading-relaxed">{rec.reasoning}</p>
      {rec.benchmark_titles.length > 0 && (
        <div className="flex items-center gap-1.5 flex-wrap">
          <BookOpenIcon className="size-3 text-muted-foreground" />
          {rec.benchmark_titles.map((title) => (
            <Badge key={title} variant="secondary" className="text-[10px] font-normal">
              {title}
            </Badge>
          ))}
        </div>
      )}
    </div>
  );
}

export function TrendsPage() {
  const {
    currentResult,
    history,
    scanning,
    error,
    scan,
    remove,
    viewHistoryItem,
  } = useRadar();
  const { t } = useI18n();

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <RadarIcon />
            {t.trends.title}
          </PageTitle>
          <PageDescription>{t.trends.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button
            onClick={scan}
            disabled={scanning}
            size="sm"
            className="gap-2"
          >
            {scanning ? <Spinner className="size-4" /> : <ScanSearchIcon className="size-4" />}
            {scanning ? t.trends.scanning : t.trends.scan}
          </Button>
        </PageActions>
      </PageHeader>

      {currentResult && (
        <div className="flex flex-col gap-6">
          <div className="rounded-[var(--radius-6)] border bg-card p-5 flex flex-col gap-3">
            <div className="flex items-center gap-2">
              <TrendingUpIcon className="size-4 text-muted-foreground" />
              <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground">
                {t.trends.summary}
              </h3>
            </div>
            <p className="text-sm leading-relaxed whitespace-pre-wrap text-card-foreground">
              {currentResult.market_summary}
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {currentResult.recommendations.map((rec, i) => (
              <RecommendationCard key={i} rec={rec} />
            ))}
          </div>
        </div>
      )}

      {!currentResult && !scanning && !error && (
        <EmptyState
          icon={<RadarIcon />}
          title={t.trends.empty}
          description={t.trends.emptyHint}
        />
      )}

      {history.length > 0 && (
        <div className="rounded-[var(--radius-6)] border bg-card p-5 flex flex-col gap-3">
          <div className="flex items-center gap-2">
            <ClockIcon className="size-4 text-muted-foreground" />
            <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground">
              {t.trends.history}
            </h3>
          </div>
          <div className="flex flex-col gap-2">
            {history.slice(0, 10).map((scan) => (
              <div key={scan.id} className="flex items-center gap-2">
                <button
                  onClick={() => viewHistoryItem(scan)}
                  className="flex-1 rounded-[var(--radius-4)] border border-[var(--border-neutral-l1)] px-3 py-2 text-left text-xs hover:bg-[var(--bg-overlay-l2)] transition-colors flex flex-col gap-1"
                >
                  <div className="font-medium text-foreground">
                    {new Date(scan.created_at).toLocaleString()}
                  </div>
                  <div className="line-clamp-1 text-muted-foreground">
                    {scan.market_summary || t.common.noSummary}
                  </div>
                </button>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="icon" className="size-8 shrink-0">
                      <MoreVerticalIcon className="size-3" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem
                      onClick={() => remove(scan.id)}
                      className="text-destructive"
                    >
                      <Trash2Icon className="size-3" />
                      <span>{t.trends.delete}</span>
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            ))}
          </div>
        </div>
      )}
    </PageContainer>
  );
}
