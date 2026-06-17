import { useState, useEffect } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
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
import { Spinner } from "@/components/ui/spinner";
import { SearchIcon, DownloadIcon, BookOpenIcon, CheckCircleIcon } from "lucide-react";
import * as novelService from "@/services/novel";
import type { BookSource, SearchBookResult } from "@/types";

export function NovelDownloadPanel() {
  const [sources, setSources] = useState<BookSource[]>([]);
  const [selectedSource, setSelectedSource] = useState<string>("");
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
      if (data.length > 0 && !selectedSource) {
        setSelectedSource(data[0].name);
      }
    } catch (err) {
      console.error("Failed to load sources:", err);
    }
  }

  async function loadLocalNovels() {
    try {
      const data = await novelService.listLocalNovels();
      setLocalNovels(data);
    } catch (err) {
      console.error("Failed to load local novels:", err);
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
    } catch (err) {
      console.error("Search failed:", err);
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
      loadLocalNovels();
    } catch (err) {
      console.error("Download failed:", err);
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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <BookOpenIcon className="h-5 w-5" />
          小说下载
        </CardTitle>
        <CardDescription>
          从书源搜索并下载小说
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex gap-2">
          <Select value={selectedSource} onValueChange={setSelectedSource}>
            <SelectTrigger className="w-[180px]">
              <SelectValue placeholder="选择书源" />
            </SelectTrigger>
            <SelectContent>
              {sources.map((source) => (
                <SelectItem key={source.name} value={source.name}>
                  {source.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            placeholder="输入书名搜索..."
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            onKeyDown={handleKeyDown}
            className="flex-1"
          />
          <Button onClick={handleSearch} disabled={searching || !keyword.trim()}>
            {searching ? <Spinner className="h-4 w-4 mr-2" /> : <SearchIcon className="h-4 w-4 mr-2" />}
            搜索
          </Button>
        </div>

        {results.length > 0 && (
          <div className="border rounded-lg">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>书名</TableHead>
                  <TableHead>作者</TableHead>
                  <TableHead>分类</TableHead>
                  <TableHead>最新章节</TableHead>
                  <TableHead>操作</TableHead>
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
                    <TableCell>
                      {downloadComplete === result.book_name ? (
                        <Badge variant="default" className="bg-green-600">
                          <CheckCircleIcon className="h-3 w-3 mr-1" />
                          已下载
                        </Badge>
                      ) : localNovels.some((n) => n.includes(result.book_name.replace(/[^\w\u4e00-\u9fa5]/g, "_"))) ? (
                        <Badge variant="secondary">已存在</Badge>
                      ) : (
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => handleDownload(result)}
                          disabled={downloading === result.url}
                        >
                          {downloading === result.url ? (
                            <Spinner className="h-3 w-3 mr-1" />
                          ) : (
                            <DownloadIcon className="h-3 w-3 mr-1" />
                          )}
                          下载
                        </Button>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}

        {localNovels.length > 0 && (
          <div className="mt-4">
            <h4 className="text-sm font-medium mb-2">已下载的小说 ({localNovels.length})</h4>
            <div className="flex flex-wrap gap-2">
              {localNovels.map((novel) => (
                <Badge key={novel} variant="secondary">
                  {novel}
                </Badge>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
