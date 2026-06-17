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
import { DownloadIcon, SearchIcon, CheckCircleIcon, BookOpenIcon } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import * as novelService from "@/services/novel";
import type { BookSource, SearchBookResult } from "@/types";

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
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <DownloadIcon />
            {t.novels.title}
          </h1>
          <p className="text-sm text-muted-foreground">{t.novels.description}</p>
        </div>
      </div>

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

      {searching && (
        <div className="flex items-center justify-center py-8">
          <Spinner className="size-6" />
        </div>
      )}

      {!searching && results.length > 0 && (
        <div className="border rounded-lg">
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
                <TableRow key={result.url}>
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
                      <Badge variant="default" className="bg-green-600">
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
        <div className="text-center py-8 text-muted-foreground">
          <SearchIcon className="size-12 mx-auto mb-4 opacity-50" />
          <p>{t.novels.download.noResults}</p>
        </div>
      )}

      {localNovels.length > 0 && (
        <div>
          <h4 className="text-sm font-medium mb-3">
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
    </div>
  );
}
