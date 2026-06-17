import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  PlusIcon,
  FolderOpenIcon,
  BookOpenIcon,
  Trash2Icon,
  MessageSquareIcon,
  ClockIcon,
} from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { useWorkspacePage } from "@/hooks/useWorkspacePage";
import { useAgentStore } from "@/stores/agent";
import { useEffect } from "react";

export function WorkspacePage() {
  const { t } = useI18n();
  const {
    workspaces,
    dialogOpen,
    setDialogOpen,
    name,
    setName,
    path,
    loading,
    handlePickDirectory,
    handleCreate,
    handleDelete,
  } = useWorkspacePage();
  const { sessions, loadSessions } = useAgentStore();

  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  const recentSessions = sessions.slice(0, 10);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <FolderOpenIcon />
            {t.sidebar.workspaces}
          </h1>
          <p className="text-sm text-muted-foreground">
            {t.sidebar.workspaceCount.replace("{count}", String(workspaces.length))}
          </p>
        </div>
        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogTrigger asChild>
            <Button>
              <PlusIcon data-icon="inline-start" />
              <span>{t.sidebar.newWorkspace}</span>
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{t.sidebar.createWorkspace}</DialogTitle>
              <DialogDescription>
                {t.sidebar.createWorkspaceDesc}
              </DialogDescription>
            </DialogHeader>
            <FieldGroup>
              <Field>
                <FieldLabel>{t.agents.name}</FieldLabel>
                <Input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder={t.sidebar.workspaceNamePlaceholder}
                />
              </Field>
              <Field>
                <FieldLabel>{t.sidebar.workspace}</FieldLabel>
                <div className="flex gap-2">
                  <Input
                    value={path}
                    onChange={(e) => setName(e.target.value)}
                    placeholder={t.sidebar.selectDirectory}
                    readOnly
                  />
                  <Button variant="outline" onClick={handlePickDirectory}>
                    <FolderOpenIcon />
                  </Button>
                </div>
              </Field>
            </FieldGroup>
            <DialogFooter>
              <Button variant="outline" onClick={() => setDialogOpen(false)}>
                {t.common.cancel}
              </Button>
              <Button onClick={handleCreate} disabled={!name || !path || loading}>
                {t.common.create}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>

      {workspaces.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <FolderOpenIcon />
            </EmptyMedia>
            <EmptyTitle>{t.sidebar.noWorkspaces}</EmptyTitle>
            <EmptyDescription>
              {t.sidebar.createWorkspaceDesc}
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={() => setDialogOpen(true)}>
              <PlusIcon data-icon="inline-start" />
              {t.sidebar.newWorkspace}
            </Button>
          </EmptyContent>
        </Empty>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {workspaces.map((ws) => (
            <Card key={ws.id} className="group transition-shadow hover:shadow-md">
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <CardTitle className="truncate text-lg">{ws.name}</CardTitle>
                    <CardDescription className="mt-1 truncate text-xs">
                      {ws.path}
                    </CardDescription>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="opacity-0 group-hover:opacity-100 transition-opacity"
                    onClick={() => handleDelete(ws.id)}
                  >
                    <Trash2Icon />
                  </Button>
                </div>
              </CardHeader>
              <CardContent>
                <Badge variant="secondary" className="text-xs">
                  <BookOpenIcon data-icon="inline-start" size={12} />
                  {t.sidebar.workspace}
                </Badge>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Conversation History */}
      <div className="mt-8">
        <div className="flex items-center gap-2 mb-4">
          <MessageSquareIcon className="size-5" />
          <h2 className="text-lg font-semibold">{t.agentChat.title}</h2>
        </div>
        {recentSessions.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t.agentChat.emptyTitle}</p>
        ) : (
          <div className="space-y-2">
            {recentSessions.map((session) => (
              <Card key={session.id} className="hover:shadow-md transition-shadow">
                <CardContent className="p-4">
                  <div className="flex items-center justify-between">
                    <div className="flex-1 min-w-0">
                      <p className="font-medium truncate">{session.title || t.agentChat.unnamedSession}</p>
                      <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
                        <span>{session.message_count} {t.agentChat.messageCount}</span>
                        <span className="flex items-center gap-1">
                          <ClockIcon className="size-3" />
                          {new Date(session.updated_at).toLocaleString()}
                        </span>
                      </div>
                    </div>
                    <Badge variant="secondary" className="text-xs">
                      {session.status}
                    </Badge>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
