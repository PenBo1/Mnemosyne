/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 归档管理设置 - 搜索、恢复或永久删除已归档的工作区和会话
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useEffect, useMemo } from "react";
import { toast } from "sonner";
import {
  ArchiveIcon,
  RotateCcwIcon,
  Trash2Icon,
  SearchIcon,
  FolderIcon,
  MessageSquareIcon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from "@/components/ui/dialog";
import { Empty, EmptyMedia, EmptyTitle, EmptyDescription } from "@/components/ui/empty";
import {
  fetchArchivedWorkspaces,
  restoreWorkspace,
  deleteWorkspace,
} from "@/features/workspace/services";
import {
  listArchivedSessions,
  restoreSession,
  deleteSession,
} from "@/features/session/services";
import type { Workspace } from "@/features/workspace/types";
import type { Session } from "@/types/session";

export function ArchiveSettings() {
  const { t } = useI18n();
  const [searchQuery, setSearchQuery] = useState("");
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [sessions, setSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(true);
  const [deleteTarget, setDeleteTarget] = useState<{
    type: "workspace" | "session";
    id: string;
    name: string;
  } | null>(null);

  useEffect(() => {
    loadArchivedItems();
  }, []);

  const loadArchivedItems = async () => {
    setLoading(true);
    try {
      const [ws, ss] = await Promise.all([
        fetchArchivedWorkspaces(),
        listArchivedSessions(),
      ]);
      setWorkspaces(ws);
      setSessions(ss);
    } catch (error) {
      toast.error("Failed to load archived items");
    } finally {
      setLoading(false);
    }
  };

  const handleRestoreWorkspace = async (id: string) => {
    try {
      await restoreWorkspace(id);
      toast.success(t.settings.restoreSuccess);
      loadArchivedItems();
    } catch (error) {
      toast.error("Failed to restore workspace");
    }
  };

  const handleDeleteWorkspace = async (id: string) => {
    try {
      await deleteWorkspace(id);
      toast.success("Workspace deleted permanently");
      loadArchivedItems();
    } catch (error) {
      toast.error("Failed to delete workspace");
    }
  };

  const handleRestoreSession = async (id: string) => {
    try {
      await restoreSession(id);
      toast.success(t.settings.restoreSuccess);
      loadArchivedItems();
    } catch (error) {
      toast.error("Failed to restore session");
    }
  };

  const handleDeleteSession = async (id: string) => {
    try {
      await deleteSession(id);
      toast.success("Session deleted permanently");
      loadArchivedItems();
    } catch (error) {
      toast.error("Failed to delete session");
    }
  };

  const filteredWorkspaces = useMemo(() => {
    if (!searchQuery.trim()) return workspaces;
    const query = searchQuery.toLowerCase();
    return workspaces.filter(
      (ws) =>
        ws.name.toLowerCase().includes(query) ||
        ws.path.toLowerCase().includes(query)
    );
  }, [workspaces, searchQuery]);

  const filteredSessions = useMemo(() => {
    if (!searchQuery.trim()) return sessions;
    const query = searchQuery.toLowerCase();
    return sessions.filter(
      (s) =>
        s.title.toLowerCase().includes(query) ||
        (s.summary && s.summary.toLowerCase().includes(query))
    );
  }, [sessions, searchQuery]);

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    if (deleteTarget.type === "workspace") {
      await handleDeleteWorkspace(deleteTarget.id);
    } else {
      await handleDeleteSession(deleteTarget.id);
    }
    setDeleteTarget(null);
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold flex items-center gap-2">
          <ArchiveIcon className="size-6" />
          {t.settings.archiveTitle}
        </h2>
        <p className="text-muted-foreground mt-1">{t.settings.archiveDesc}</p>
      </div>

      <div className="relative">
        <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
        <Input
          placeholder={t.settings.searchArchive}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="pl-10"
        />
      </div>

      <Tabs defaultValue="workspaces" className="w-full">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="workspaces" className="flex items-center gap-2">
            <FolderIcon className="size-4" />
            {t.settings.archivedWorkspaces} ({filteredWorkspaces.length})
          </TabsTrigger>
          <TabsTrigger value="sessions" className="flex items-center gap-2">
            <MessageSquareIcon className="size-4" />
            {t.settings.archivedSessions} ({filteredSessions.length})
          </TabsTrigger>
        </TabsList>

        <TabsContent value="workspaces" className="mt-4">
          {loading ? (
            <div className="text-center py-8 text-muted-foreground">
              Loading...
            </div>
          ) : filteredWorkspaces.length === 0 ? (
            <Empty>
              <EmptyMedia variant="icon">
                <ArchiveIcon className="size-4" />
              </EmptyMedia>
              <EmptyTitle>{t.settings.noArchivedItems}</EmptyTitle>
              <EmptyDescription>No archived workspaces found</EmptyDescription>
            </Empty>
          ) : (
            <div className="space-y-3">
              {filteredWorkspaces.map((ws) => (
                <Card key={ws.id}>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-lg">{ws.name}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="flex items-center justify-between">
                      <div className="text-sm text-muted-foreground space-y-1">
                        <div className="truncate max-w-md">{ws.path}</div>
                        {ws.archived_at && (
                          <div>Archived: {new Date(ws.archived_at).toLocaleString()}</div>
                        )}
                      </div>
                      <div className="flex gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleRestoreWorkspace(ws.id)}
                        >
                          <RotateCcwIcon className="size-4 mr-1" />
                          {t.settings.restore}
                        </Button>
                        <Button
                          variant="destructive"
                          size="sm"
                          onClick={() =>
                            setDeleteTarget({
                              type: "workspace",
                              id: ws.id,
                              name: ws.name,
                            })
                          }
                        >
                          <Trash2Icon className="size-4 mr-1" />
                          {t.settings.permanentDelete}
                        </Button>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="sessions" className="mt-4">
          {loading ? (
            <div className="text-center py-8 text-muted-foreground">
              Loading...
            </div>
          ) : filteredSessions.length === 0 ? (
            <Empty>
              <EmptyMedia variant="icon">
                <ArchiveIcon className="size-4" />
              </EmptyMedia>
              <EmptyTitle>{t.settings.noArchivedItems}</EmptyTitle>
              <EmptyDescription>No archived sessions found</EmptyDescription>
            </Empty>
          ) : (
            <div className="space-y-3">
              {filteredSessions.map((s) => (
                <Card key={s.id}>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-lg">
                      {s.title || "Untitled Session"}
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="flex items-center justify-between">
                      <div className="text-sm text-muted-foreground space-y-1">
                        <div>
                          {s.message_count} messages • {s.input_tokens + s.output_tokens} tokens
                        </div>
                        <div>
                          Updated: {new Date(s.updated_at).toLocaleString()}
                        </div>
                      </div>
                      <div className="flex gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleRestoreSession(s.id)}
                        >
                          <RotateCcwIcon className="size-4 mr-1" />
                          {t.settings.restore}
                        </Button>
                        <Button
                          variant="destructive"
                          size="sm"
                          onClick={() =>
                            setDeleteTarget({
                              type: "session",
                              id: s.id,
                              name: s.title || "Untitled Session",
                            })
                          }
                        >
                          <Trash2Icon className="size-4 mr-1" />
                          {t.settings.permanentDelete}
                        </Button>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </TabsContent>
      </Tabs>

      <Dialog
        open={!!deleteTarget}
        onOpenChange={() => setDeleteTarget(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t.settings.deleteConfirmTitle}</DialogTitle>
            <DialogDescription>
              {t.settings.deleteConfirmDesc}
              <br />
              <strong>{deleteTarget?.name}</strong>
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose>{t.common.cancel}</DialogClose>
            <Button variant="destructive" onClick={confirmDelete}>
              {t.settings.permanentDelete}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}