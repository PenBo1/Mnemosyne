import { useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Field, FieldGroup, FieldLabel, FieldSeparator } from "@/components/ui/field";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  MoreVerticalIcon,
  PencilIcon,
  BotIcon,
  ShieldCheckIcon,
  Building2Icon,
  ListTodoIcon,
  LayersIcon,
  PenLineIcon,
  SearchIcon,
  RefreshCwIcon,
  EyeIcon,
  RotateCcwIcon,
  BookOpenIcon,
  BrainCircuitIcon,
  ScrollTextIcon,
  SaveIcon,
} from "lucide-react";
<<<<<<< Updated upstream
import { useI18n } from "@/lib/i18n";
import { useAgents } from "@/hooks/useAgents";
import type { Agent, AgentIdentity } from "@/types";
=======
import { useI18n } from "@/shared/i18n";
import { useAgents } from "@/features/chat/hooks/useAgents";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { LoadingState } from "@/components/shared/state";
import type { Agent } from "@/shared/types";
>>>>>>> Stashed changes
import type { LucideIcon } from "lucide-react";

const ROLE_ICONS: Record<string, LucideIcon> = {
  architect: Building2Icon,
  planner: ListTodoIcon,
  composer: LayersIcon,
  writer: PenLineIcon,
  auditor: SearchIcon,
  reviser: RefreshCwIcon,
  observer: EyeIcon,
  reflector: RotateCcwIcon,
};

export function AgentsSettings() {
  const { t } = useI18n();
  const { agents, models, loading, update, toggleStatus, getIdentity, updateIdentity } = useAgents();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingAgent, setEditingAgent] = useState<Agent | null>(null);
  const [model, setModel] = useState("gpt-4");
  const [temperature, setTemperature] = useState([0.7]);
  const [maxTokens, setMaxTokens] = useState("2048");

  const [identityOpen, setIdentityOpen] = useState(false);
  const [identityAgent, setIdentityAgent] = useState<Agent | null>(null);
  const [identity, setIdentity] = useState<AgentIdentity | null>(null);
  const [identityLoading, setIdentityLoading] = useState(false);
  const [identitySaving, setIdentitySaving] = useState(false);
  const [identityTab, setIdentityTab] = useState("soul");
  const [identityDraft, setIdentityDraft] = useState({ soul: "", context: "", memory: "" });

  function openEdit(agent: Agent) {
    setEditingAgent(agent);
    setModel(agent.model);
    setTemperature([agent.temperature]);
    setMaxTokens(String(agent.maxTokens));
    setDialogOpen(true);
  }

  const openIdentity = useCallback(async (agent: Agent) => {
    setIdentityAgent(agent);
    setIdentityOpen(true);
    setIdentityLoading(true);
    setIdentityTab("soul");
    try {
      const data = await getIdentity(agent.id);
      setIdentity(data);
      setIdentityDraft({
        soul: data?.soul ?? "",
        context: data?.context ?? "",
        memory: data?.memory ?? "",
      });
    } finally {
      setIdentityLoading(false);
    }
  }, [getIdentity]);

  async function handleSave() {
    if (!editingAgent) return;
    await update(editingAgent.id, {
      model,
      temperature: temperature[0],
      maxTokens: Number(maxTokens),
    });
    setDialogOpen(false);
  }

  async function handleIdentitySave() {
    if (!identityAgent) return;
    setIdentitySaving(true);
    try {
      const updated = await updateIdentity(identityAgent.id, identityDraft);
      if (updated) {
        setIdentity(updated);
      }
    } finally {
      setIdentitySaving(false);
    }
  }

  const hasIdentityChanges = identity && (
    identityDraft.soul !== identity.soul ||
    identityDraft.context !== identity.context ||
    identityDraft.memory !== identity.memory
  );

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.agents}</PageTitle>
          <PageDescription>{t.agents.pipelineDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

<<<<<<< Updated upstream
      <div className="rounded-lg border bg-card">
        <div className="px-4 py-3">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <ShieldCheckIcon className="size-3.5" />
            <span>{t.agents.systemNote}</span>
          </div>
        </div>
      </div>
=======
      {/* 系统提示 */}
      <Card>
        <CardContent className="flex items-center gap-2 text-xs text-muted-foreground">
          <ShieldCheckIcon className="size-3.5 shrink-0" />
          <span>{t.agents.systemNote}</span>
        </CardContent>
      </Card>
>>>>>>> Stashed changes

      {loading && <LoadingState label={t.common.loading} />}

<<<<<<< Updated upstream
      <div className="rounded-lg border bg-card divide-y">
        {agents.map((agent) => {
          const Icon = ROLE_ICONS[agent.id] || BotIcon;
          return (
            <div
              key={agent.id}
              className="px-4 py-3 cursor-pointer hover:bg-muted/50 transition-colors"
              onClick={() => openIdentity(agent)}
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <Icon className="size-4 shrink-0" />
                  <span className="text-sm font-medium">{agent.name}</span>
                  <Badge variant="secondary" className="text-xs">{agent.model}</Badge>
                  <Badge variant={agent.status === "active" ? "default" : "outline"} className="text-xs">
                    {agent.status === "active" ? t.agents.status.active : t.agents.status.inactive}
                  </Badge>
                </div>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="icon-sm" onClick={(e) => e.stopPropagation()}>
                      <MoreVerticalIcon />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem onClick={(e) => { e.stopPropagation(); toggleStatus(agent.id); }}>
                      {agent.status === "active" ? t.agents.deactivate : t.agents.activate}
                    </DropdownMenuItem>
                    <DropdownMenuItem onClick={(e) => { e.stopPropagation(); openEdit(agent); }}>
                      <PencilIcon />
                      <span>{t.agents.configure}</span>
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
=======
      {/* Agent 列表 */}
      <Card className="py-0 gap-0">
        <CardContent className="divide-y px-0">
          {agents.map((agent) => {
            const Icon = ROLE_ICONS[agent.id] || BotIcon;
            return (
              <div key={agent.id} className="flex flex-col gap-2 px-4 py-3 transition-colors hover:bg-muted/50">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Icon className="size-4 shrink-0" />
                    <span className="text-sm font-medium">{agent.name}</span>
                    <Badge variant="secondary" className="text-xs">{agent.model}</Badge>
                    <Badge variant={agent.status === "active" ? "default" : "outline"} className="text-xs">
                      {agent.status === "active" ? t.agents.status.active : t.agents.status.inactive}
                    </Badge>
                  </div>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon-sm">
                        <MoreVerticalIcon />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={() => toggleStatus(agent.id)}>
                        {agent.status === "active" ? t.agents.deactivate : t.agents.activate}
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => openEdit(agent)}>
                        <PencilIcon />
                        <span>{t.agents.configure}</span>
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
                <p className="line-clamp-2 text-xs text-muted-foreground">{agent.description}</p>
                <div className="flex items-center gap-4 text-xs text-muted-foreground">
                  <span>{t.agents.temperature}: {agent.temperature}</span>
                  <span>{t.agents.maxTokens}: {agent.maxTokens}</span>
                </div>
>>>>>>> Stashed changes
              </div>
            );
          })}
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{t.agents.configure} {editingAgent?.name}</DialogTitle>
            <DialogDescription>
              {t.agents.pipelineDesc}
            </DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.agents.model}</FieldLabel>
              <Select value={model} onValueChange={setModel}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {models.map((m) => (
                    <SelectItem key={m} value={m}>{m}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <FieldSeparator />
            <Field>
              <FieldLabel>{t.agents.temperature}: {temperature[0]}</FieldLabel>
              <Slider value={temperature} onValueChange={setTemperature} min={0} max={2} step={0.1} />
            </Field>
            <Field>
              <FieldLabel>{t.agents.maxTokens}</FieldLabel>
              <Input
                type="number"
                value={maxTokens}
                onChange={(e) => setMaxTokens(e.target.value)}
                min={256}
                max={32768}
              />
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              {t.agents.cancel}
            </Button>
            <Button onClick={handleSave}>
              {t.common.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
<<<<<<< Updated upstream

      <Dialog open={identityOpen} onOpenChange={setIdentityOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              {identityAgent && (() => {
                const Icon = ROLE_ICONS[identityAgent.id] || BotIcon;
                return <Icon className="size-5" />;
              })()}
              {identityAgent?.name}
            </DialogTitle>
            <DialogDescription>
              {t.agents.identityDesc}
            </DialogDescription>
          </DialogHeader>

          {identityLoading ? (
            <div className="flex items-center justify-center py-12">
              <Spinner className="size-6" />
            </div>
          ) : (
            <Tabs value={identityTab} onValueChange={setIdentityTab}>
              <TabsList className="grid w-full grid-cols-3">
                <TabsTrigger value="soul" className="flex items-center gap-1.5">
                  <BrainCircuitIcon className="size-3.5" />
                  {t.agents.tabs.soul}
                </TabsTrigger>
                <TabsTrigger value="context" className="flex items-center gap-1.5">
                  <ScrollTextIcon className="size-3.5" />
                  {t.agents.tabs.context}
                </TabsTrigger>
                <TabsTrigger value="memory" className="flex items-center gap-1.5">
                  <BookOpenIcon className="size-3.5" />
                  {t.agents.tabs.memory}
                </TabsTrigger>
              </TabsList>

              <TabsContent value="soul" className="mt-3">
                <div className="flex flex-col gap-2">
                  <label className="text-sm font-medium">{t.agents.identity.soulLabel}</label>
                  <p className="text-xs text-muted-foreground">{t.agents.identity.soulDesc}</p>
                  <Textarea
                    value={identityDraft.soul}
                    onChange={(e) => setIdentityDraft((d) => ({ ...d, soul: e.target.value }))}
                    placeholder={t.agents.identity.soulPlaceholder}
                    className="min-h-[280px] max-h-[400px] font-mono text-sm resize-none"
                  />
                </div>
              </TabsContent>

              <TabsContent value="context" className="mt-3">
                <div className="flex flex-col gap-2">
                  <label className="text-sm font-medium">{t.agents.identity.contextLabel}</label>
                  <p className="text-xs text-muted-foreground">{t.agents.identity.contextDesc}</p>
                  <Textarea
                    value={identityDraft.context}
                    onChange={(e) => setIdentityDraft((d) => ({ ...d, context: e.target.value }))}
                    placeholder={t.agents.identity.contextPlaceholder}
                    className="min-h-[280px] max-h-[400px] font-mono text-sm resize-none"
                  />
                </div>
              </TabsContent>

              <TabsContent value="memory" className="mt-3">
                <div className="flex flex-col gap-2">
                  <label className="text-sm font-medium">{t.agents.identity.memoryLabel}</label>
                  <p className="text-xs text-muted-foreground">{t.agents.identity.memoryDesc}</p>
                  <Textarea
                    value={identityDraft.memory}
                    onChange={(e) => setIdentityDraft((d) => ({ ...d, memory: e.target.value }))}
                    placeholder={t.agents.identity.memoryPlaceholder}
                    className="min-h-[280px] max-h-[400px] font-mono text-sm resize-none"
                  />
                </div>
              </TabsContent>
            </Tabs>
          )}

          <DialogFooter>
            <Button variant="outline" onClick={() => setIdentityOpen(false)}>
              {t.agents.cancel}
            </Button>
            <Button onClick={handleIdentitySave} disabled={identitySaving || !hasIdentityChanges}>
              {identitySaving ? (
                <Spinner className="size-4 mr-1" />
              ) : (
                <SaveIcon className="size-4 mr-1" />
              )}
              {t.common.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
=======
    </PageContainer>
>>>>>>> Stashed changes
  );
}
