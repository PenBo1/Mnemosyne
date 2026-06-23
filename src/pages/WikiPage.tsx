import { useState, useEffect } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";
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
import { useWiki } from "@/hooks/useWiki";
import { WikiEntryEditor } from "@/components/WikiEntryEditor";
import { WikiGraphViewComponent } from "@/components/WikiGraphView";
import type { WikiEntry, WikiCategory, CreateWikiEntryRequest, UpdateWikiEntryRequest } from "@/types";

const WIKI_CATEGORIES: WikiCategory[] = ["character", "location", "event", "concept", "item", "other"];

export function WikiPage({ novelId }: { novelId: string }) {
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
    loadEntries();
    loadGraph();
  }, [loadEntries, loadGraph]);

  const filteredEntries = entries.filter((entry) => {
    const matchesCategory = filterCategory === "all" || entry.category === filterCategory;
    const matchesSearch = !searchQuery || 
      entry.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      entry.content.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const handleCreate = async (request: CreateWikiEntryRequest | UpdateWikiEntryRequest) => {
    await createEntry(request as CreateWikiEntryRequest);
    setIsCreating(false);
  };

  const handleUpdate = async (request: UpdateWikiEntryRequest) => {
    if (editingEntry) {
      await updateEntry(editingEntry.id, request);
      setEditingEntry(null);
    }
  };

  const handleDelete = async (id: string) => {
    await deleteEntry(id);
  };

  const handleSearch = async () => {
    if (searchQuery.trim()) {
      await search(searchQuery.trim());
    } else {
      await loadEntries();
    }
  };

  const categoryCounts = entries.reduce(
    (acc, entry) => {
      acc[entry.category] = (acc[entry.category] || 0) + 1;
      return acc;
    },
    {} as Record<WikiCategory, number>,
  );

  return (
    <div className="flex flex-col gap-6 h-full">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <BookOpenIcon />
            Repo Wiki
          </h1>
          <p className="text-sm text-muted-foreground">
            Knowledge base for your novel
          </p>
        </div>
        <Button onClick={() => setIsCreating(true)}>
          <PlusIcon data-icon="inline-start" />
          New Entry
        </Button>
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as "list" | "graph")} className="flex-1 flex flex-col">
        <TabsList className="mb-2">
          <TabsTrigger value="list">
            List ({entries.length})
          </TabsTrigger>
          <TabsTrigger value="graph">
            <NetworkIcon className="size-4 mr-1" />
            Graph
          </TabsTrigger>
        </TabsList>

        {/* List View */}
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
              {/* Search & Filter */}
              <div className="flex items-center gap-3">
                <div className="relative flex-1 max-w-sm">
                  <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
                  <Input
                    placeholder="Search entries..."
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
                    All
                    <Badge variant="outline" className="ml-1.5 size-5 justify-center text-xs">
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
                        {cat}
                        <Badge variant="outline" className="ml-1.5 size-5 justify-center text-xs">
                          {count}
                        </Badge>
                      </Button>
                    );
                  })}
                </div>
              </div>

              {/* Entry List */}
              {loading ? (
                <div className="flex items-center justify-center py-12">
                  <Spinner className="size-6" />
                </div>
              ) : filteredEntries.length === 0 ? (
                <Empty>
                  <EmptyHeader>
                    <EmptyMedia variant="icon">
                      <BookOpenIcon />
                    </EmptyMedia>
                    <EmptyTitle>No entries yet</EmptyTitle>
                    <EmptyDescription>Create your first wiki entry</EmptyDescription>
                  </EmptyHeader>
                  <EmptyContent>
                    <Button onClick={() => setIsCreating(true)}>
                      <PlusIcon data-icon="inline-start" />
                      New Entry
                    </Button>
                  </EmptyContent>
                </Empty>
              ) : (
                <ScrollArea className="flex-1">
                  <Card>
                    <CardContent className="p-0">
                      <div className="divide-y">
                        {filteredEntries.map((entry) => (
                          <div
                            key={entry.id}
                            className="flex items-start gap-4 px-4 py-3 hover:bg-muted/50 transition-colors cursor-pointer"
                            onClick={() => setEditingEntry(entry)}
                          >
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <span className="font-medium truncate">{entry.title}</span>
                                <Badge variant="secondary" className="shrink-0 text-xs capitalize">
                                  {entry.category}
                                </Badge>
                                {entry.importance >= 5 && (
                                  <Badge variant="outline" className="shrink-0 text-xs">
                                    ★ {entry.importance}
                                  </Badge>
                                )}
                              </div>
                              <p className="mt-1 text-sm text-muted-foreground line-clamp-2 whitespace-pre-wrap">
                                {entry.content}
                              </p>
                              {entry.tags.length > 0 && (
                                <div className="mt-2 flex items-center gap-1.5 flex-wrap">
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
                                  <span>Edit</span>
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    handleDelete(entry.id);
                                  }}
                                  className="text-destructive"
                                >
                                  <Trash2Icon />
                                  <span>Delete</span>
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

        {/* Graph View */}
        <TabsContent value="graph" className="flex-1 mt-0">
          {graph ? (
            <WikiGraphViewComponent
              graph={graph}
              entries={entries}
              onNodeClick={(entry) => setEditingEntry(entry)}
            />
          ) : (
            <Empty>
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <NetworkIcon />
                </EmptyMedia>
                <EmptyTitle>No graph data</EmptyTitle>
                <EmptyDescription>Create entries to see the knowledge graph</EmptyDescription>
              </EmptyHeader>
            </Empty>
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}