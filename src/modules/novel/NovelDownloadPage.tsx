import { useState, useEffect } from "react";
import { toast } from "sonner";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { EmptyState, LoadingState } from "@/components/shared/state";
import { DownloadIcon, SearchIcon, CheckCircleIcon, BookOpenIcon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import * as novelService from "@/features/novel/services";
import type { BookSource, SearchBookResult } from "@/shared/types";

export function NovelDownloadPage() {
  const { t } = useI18n();
  const [sources, setSources] = useState<BookSource[]>([]);
  const [selectedSource, setSelectedSource] = useState<string>("all");
  const [keyword, setKeyword] = useState("");
  const [results, setResults] = useState<SearchBookResult[]>([]);
  const [localNovels, setLocalNovels] = useState<string[]>([]);
  const [searching, setSearching] = useState(false);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [downloadComplete, setDownloadComplete] = useState<string | null>(null);

  useEffect(() => {
    loadSources();
    loadLocalNovels();
  }, []);

  async function loadSources() {
    try {
      const data = await novelService.listBookSources();
      setSources(data.filter((s) => !s.disabled && s.search && !s.search.disabled));
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToLoad);
    }
  }

  async function loadLocalNovels() {
    try {
      const data = await novelService.listLocalNovels();
      setLocalNovels(data);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToLoad);
    }
  }

  async function handleSearch() {
    if (!selectedSource || !keyword.trim()) return;
    setSearching(true);
    setResults([]);
    setDownloadComplete(null);
    try {
      const data = await novelService.searchNovels(selectedSource, keyword.trim());
      setResults(data);
      if (data.length === 0) {
        toast.info(t.novels.download.noResults);
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToLoad);
    } finally {
      setSearching(false);
    }
  }

  async function handleDownload(result: SearchBookResult) {
    setDownloading(result.url);
    setDownloadComplete(null);
    try {
      await novelService.downloadNovel(result.source_name, result.url, result.book_name);
      setDownloadComplete(result.book_name);
      toast.success(t.novels.download.downloaded);
      loadLocalNovels();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t.common.failedToCreate);
    } finally {
      setDownloading(null);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") {
      handleSearch();
    }
  }

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <DownloadIcon />
            {t.novels.title}
          </PageTitle>
          <PageDescription>{t.novels.description}</PageDescription>
        </PageHeading>
      </PageHeader>

      <div className="flex items-center gap-3">
        <Select value={selectedSource} onValueChange={setSelectedSource}>
          <SelectTrigger className="w-[180px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{t.novels.download.allSources}</SelectItem>
            {sources.map((source) => (
              <SelectItem key={source.name} value={source.name}>
                {source.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Input
          placeholder={t.novels.download.searchPlaceholder}
          value={keyword}
          onChange={(e) => setKeyword(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1 max-w-sm"
        />
        <Button onClick={handleSearch} disabled={searching || !keyword.trim()}>
          {searching ? <Spinner className="size-4" data-icon="inline-start" /> : <SearchIcon data-icon="inline-start" />}
          {t.novels.download.search}
        </Button>
      </div>

      {searching && <LoadingState label={t.common.loading} />}

      {!searching && results.length > 0 && (
        <div className="border rounded-[var(--radius-6)]">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t.novels.download.bookName}</TableHead>
                <TableHead>{t.novels.download.author}</TableHead>
                <TableHead>{t.novels.download.category}</TableHead>
                <TableHead>{t.novels.download.latestChapter}</TableHead>
                <TableHead className="text-right">{t.novels.download.action}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {results.map((result) => (
                <TableRow key={result.url} className="transition-colors hover:bg-[var(--bg-overlay-l2)]">
                  <TableCell className="font-medium">{result.book_name}</TableCell>
                  <TableCell>{result.author}</TableCell>
                  <TableCell>
                    {result.category && <Badge variant="outline">{result.category}</Badge>}
                  </TableCell>
                  <TableCell className="max-w-[200px] truncate text-sm text-muted-foreground">
                    {result.latest_chapter}
                  </TableCell>
                  <TableCell className="text-right">
                    {downloadComplete === result.book_name ? (
                      <Badge variant="default">
                        <CheckCircleIcon data-icon="inline-start" />
                        {t.novels.download.downloaded}
                      </Badge>
                    ) : localNovels.some((n) => n.includes(result.book_name.replace(/[^\w\u4e00-\u9fa5]/g, "_"))) ? (
                      <Badge variant="secondary">{t.novels.download.exists}</Badge>
                    ) : (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => handleDownload(result)}
                        disabled={downloading === result.url}
                      >
                        {downloading === result.url ? (
                          <Spinner className="size-3" data-icon="inline-start" />
                        ) : (
                          <DownloadIcon data-icon="inline-start" />
                        )}
                        {t.novels.download.download}
                      </Button>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      {!searching && results.length === 0 && keyword && (
        <EmptyState icon={<SearchIcon />} title={t.novels.download.noResults} />
      )}

      {localNovels.length > 0 && (
        <div className="flex flex-col gap-3">
          <h4 className="text-sm font-medium">
            {t.novels.download.downloadedNovels.replace("{count}", String(localNovels.length))}
          </h4>
          <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
            {localNovels.map((novel) => (
              <Card key={novel}>
                <CardContent className="p-3">
                  <div className="flex items-center gap-2">
                    <BookOpenIcon className="size-4 text-muted-foreground" />
                    <span className="text-sm truncate">{novel}</span>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      )}
    </PageContainer>
  );
}
