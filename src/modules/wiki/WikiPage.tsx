import { useState, useEffect, useMemo, useCallback } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  BookOpenIcon,
  PlusIcon,
  SearchIcon,
  NetworkIcon,
  PencilIcon,
  Trash2Icon,
  MoreVerticalIcon,
  TagIcon,
} from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { PageContainer, PageHeader, PageHeading, PageTitle, PageDescription, PageActions } from "@/components/shared/page-layout";
import { LoadingState, EmptyState } from "@/components/shared/state";
import { useWiki } from "@/features/wiki/hooks/useWiki";
import { WikiEntryEditor, WikiGraphViewComponent } from "@/features/wiki/components";
import { useI18n } from "@/shared/i18n";
import { useWorkspaceStore } from "@/stores/workspace";
import { fetchNovels } from "@/features/novel/services";
import type { WikiEntry, WikiCategory, CreateWikiEntryRequest, UpdateWikiEntryRequest, Novel } from "@/shared/types";

const WIKI_CATEGORIES: WikiCategory[] = ["general", "character", "location", "event", "concept", "reference"];

/**
 * WikiPage 不再接收硬编码 novelId，而是从 active workspace 自动获取
 * 该 workspace 下的第一个 novel 作为 wiki 的归属。
 *
 * 若 workspace 下没有 novel，显示空状态提示用户先创建 novel。
 */
export function WikiPage() {
  const { t } = useI18n();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const [novels, setNovels] = useState<Novel[]>([]);

  useEffect(() => {
    fetchNovels()
      .then(setNovels)
      .catch(() => setNovels([]));
  }, [activeWorkspaceId]);

  // 当前 workspace 下的第一个 novel（wiki 绑定到 novel）
  const activeNovel = useMemo(() => {
    if (!activeWorkspaceId) return undefined;
    return novels.find((n) => n.workspace_id === activeWorkspaceId);
  }, [novels, activeWorkspaceId]);

  const novelId = activeNovel?.id;
  const {
    entries,
    graph,
    loading,
    loadEntries,
    loadGraph,
    createEntry,
    updateEntry,
    deleteEntry,
    search,
  } = useWiki(novelId);

  const [activeTab, setActiveTab] = useState<"list" | "graph">("list");
  const [searchQuery, setSearchQuery] = useState("");
  const [filterCategory, setFilterCategory] = useState<WikiCategory | "all">("all");
  const [editingEntry, setEditingEntry] = useState<WikiEntry | null>(null);
  const [isCreating, setIsCreating] = useState(false);

  useEffect(() => {
    if (novelId) {
      loadEntries();
      loadGraph();
    }
  }, [loadEntries, loadGraph, novelId]);

  const filteredEntries = useMemo(() => {
    const q = searchQuery.toLowerCase();
    return entries.filter((entry) => {
      const matchesCategory = filterCategory === "all" || entry.category === filterCategory;
      const matchesSearch = !q ||
        entry.title.toLowerCase().includes(q) ||
        entry.content.toLowerCase().includes(q);
      return matchesCategory && matchesSearch;
    });
  }, [entries, filterCategory, searchQuery]);

  const handleCreate = useCallback(async (request: CreateWikiEntryRequest | UpdateWikiEntryRequest) => {
    await createEntry(request as CreateWikiEntryRequest);
    setIsCreating(false);
  }, [createEntry]);

  const handleUpdate = useCallback(async (request: UpdateWikiEntryRequest) => {
    if (editingEntry) {
      await updateEntry(editingEntry.id, request);
      setEditingEntry(null);
    }
  }, [editingEntry, updateEntry]);

  const handleDelete = useCallback(async (id: string) => {
    await deleteEntry(id);
  }, [deleteEntry]);

  const handleSearch = useCallback(async () => {
    if (searchQuery.trim()) {
      await search(searchQuery.trim());
    } else {
      await loadEntries();
    }
  }, [searchQuery, search, loadEntries]);

  const categoryCounts = useMemo(
    () =>
      entries.reduce(
        (acc, entry) => {
          acc[entry.category] = (acc[entry.category] || 0) + 1;
          return acc;
        },
        {} as Record<WikiCategory, number>,
      ),
    [entries],
  );

  // 无 active workspace 或 workspace 下无 novel 的空状态
  if (!activeWorkspaceId) {
    return (
      <PageContainer>
        <PageHeader>
          <PageHeading>
            <PageTitle>
              <BookOpenIcon />
              {t.wiki.title}
            </PageTitle>
            <PageDescription>{t.wiki.description}</PageDescription>
          </PageHeading>
        </PageHeader>
        <EmptyState
          icon={<BookOpenIcon />}
          title={t.wiki.noWorkspace}
          description={t.wiki.noWorkspaceHint}
        />
      </PageContainer>
    );
  }

  if (!novelId) {
    return (
      <PageContainer>
        <PageHeader>
          <PageHeading>
            <PageTitle>
              <BookOpenIcon />
              {t.wiki.title}
            </PageTitle>
            <PageDescription>{t.wiki.description}</PageDescription>
          </PageHeading>
        </PageHeader>
        <EmptyState
          icon={<BookOpenIcon />}
          title={t.wiki.noNovel}
          description={t.wiki.noNovelHint}
        />
      </PageContainer>
    );
  }

  return (
    <PageContainer>
      {/* 头部 */}
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <BookOpenIcon />
            {t.wiki.title}
          </PageTitle>
          <PageDescription>{t.wiki.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button onClick={() => setIsCreating(true)}>
            <PlusIcon data-icon="inline-start" />
            {t.wiki.newEntry}
          </Button>
        </PageActions>
      </PageHeader>

      {/* 标签页 */}
      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as "list" | "graph")} className="flex-1 flex flex-col gap-2">
        <TabsList>
          <TabsTrigger value="list">
            {t.wiki.listView} ({entries.length})
          </TabsTrigger>
          <TabsTrigger value="graph">
            <NetworkIcon className="size-4" />
            {t.wiki.graphView}
          </TabsTrigger>
        </TabsList>

        {/* 列表视图 */}
        <TabsContent value="list" className="flex-1 mt-0">
          {isCreating || editingEntry ? (
            <WikiEntryEditor
              entry={editingEntry}
              isNew={isCreating}
              onSave={isCreating ? handleCreate : handleUpdate}
              onCancel={() => {
                setIsCreating(false);
                setEditingEntry(null);
              }}
            />
          ) : (
            <div className="flex flex-col gap-4">
              {/* 搜索与过滤 */}
              <div className="flex items-center gap-3">
                <div className="relative flex-1 max-w-sm">
                  <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
                  <Input
                    placeholder={t.wiki.searchPlaceholder}
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                    className="pl-9"
                  />
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    variant={filterCategory === "all" ? "secondary" : "ghost"}
                    size="sm"
                    onClick={() => setFilterCategory("all")}
                  >
                    {t.wiki.all}
                    <Badge variant="outline" className="size-5 justify-center text-xs">
                      {entries.length}
                    </Badge>
                  </Button>
                  {WIKI_CATEGORIES.map((cat) => {
                    const count = categoryCounts[cat] || 0;
                    if (count === 0) return null;
                    return (
                      <Button
                        key={cat}
                        variant={filterCategory === cat ? "secondary" : "ghost"}
                        size="sm"
                        onClick={() => setFilterCategory(filterCategory === cat ? "all" : cat)}
                      >
                        {t.wiki.categories[cat]}
                        <Badge variant="outline" className="size-5 justify-center text-xs">
                          {count}
                        </Badge>
                      </Button>
                    );
                  })}
                </div>
              </div>

              {/* 条目列表 */}
              {loading ? (
                <LoadingState />
              ) : filteredEntries.length === 0 ? (
                <EmptyState
                  icon={<BookOpenIcon />}
                  title={t.wiki.empty}
                  description={t.wiki.emptyHint}
                >
                  <Button onClick={() => setIsCreating(true)}>
                    <PlusIcon data-icon="inline-start" />
                    {t.wiki.newEntry}
                  </Button>
                </EmptyState>
              ) : (
                <ScrollArea className="flex-1">
                  <Card className="transition-shadow">
                    <CardContent className="p-0">
                      <div className="divide-y">
                        {filteredEntries.map((entry) => (
                          <div
                            key={entry.id}
                            className="flex items-start gap-4 px-4 py-3 hover:bg-muted/50 transition-colors cursor-pointer"
                            onClick={() => setEditingEntry(entry)}
                          >
                            <div className="flex-1 min-w-0 flex flex-col gap-2">
                              <div className="flex items-center gap-2">
                                <span className="font-medium truncate">{entry.title}</span>
                                <Badge variant="secondary" className="shrink-0 text-xs">
                                  {t.wiki.categories[entry.category]}
                                </Badge>
                                {entry.importance >= 5 && (
                                  <Badge variant="outline" className="shrink-0 text-xs">
                                    ★ {entry.importance}
                                  </Badge>
                                )}
                              </div>
                              <p className="text-sm text-muted-foreground line-clamp-2 whitespace-pre-wrap">
                                {entry.content}
                              </p>
                              {entry.tags.length > 0 && (
                                <div className="flex items-center gap-1.5 flex-wrap">
                                  <TagIcon className="size-3 text-muted-foreground" />
                                  {entry.tags.map((tag) => (
                                    <Badge key={tag} variant="outline" className="text-xs">
                                      {tag}
                                    </Badge>
                                  ))}
                                </div>
                              )}
                            </div>
                            <DropdownMenu>
                              <DropdownMenuTrigger asChild>
                                <Button variant="ghost" size="icon-sm" className="shrink-0">
                                  <MoreVerticalIcon />
                                </Button>
                              </DropdownMenuTrigger>
                              <DropdownMenuContent align="end">
                                <DropdownMenuItem onClick={() => setEditingEntry(entry)}>
                                  <PencilIcon />
                                  <span>{t.common.edit}</span>
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    handleDelete(entry.id);
                                  }}
                                  className="text-destructive"
                                >
                                  <Trash2Icon />
                                  <span>{t.common.delete}</span>
                                </DropdownMenuItem>
                              </DropdownMenuContent>
                            </DropdownMenu>
                          </div>
                        ))}
                      </div>
                    </CardContent>
                  </Card>
                </ScrollArea>
              )}
            </div>
          )}
        </TabsContent>

        {/* 图谱视图 */}
        <TabsContent value="graph" className="flex-1 mt-0">
          {graph ? (
            <WikiGraphViewComponent
              graph={graph}
              entries={entries}
              onNodeClick={(entry) => setEditingEntry(entry)}
            />
          ) : (
            <EmptyState
              icon={<NetworkIcon />}
              title={t.wiki.noGraph}
              description={t.wiki.noGraphHint}
            />
          )}
        </TabsContent>
      </Tabs>
    </PageContainer>
  );
}
