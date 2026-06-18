import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
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
} from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { useAgents } from "@/hooks/useAgents";
import type { Agent } from "@/types";
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
  const { agents, models, loading, update, toggleStatus } = useAgents();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingAgent, setEditingAgent] = useState<Agent | null>(null);
  const [model, setModel] = useState("gpt-4");
  const [temperature, setTemperature] = useState([0.7]);
  const [maxTokens, setMaxTokens] = useState("2048");

  function openEdit(agent: Agent) {
    setEditingAgent(agent);
    setModel(agent.model);
    setTemperature([agent.temperature]);
    setMaxTokens(String(agent.maxTokens));
    setDialogOpen(true);
  }

  async function handleSave() {
    if (!editingAgent) return;
    await update(editingAgent.id, {
      model,
      temperature: temperature[0],
      maxTokens: Number(maxTokens),
    });
    setDialogOpen(false);
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">{t.settings.agents}</h1>
        <p className="text-sm text-muted-foreground">
          {t.agents.pipelineDesc}
        </p>
      </div>

      {/* System Note */}
      <div className="rounded-lg border bg-card">
        <div className="px-4 py-3">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <ShieldCheckIcon className="size-3.5" />
            <span>{t.agents.systemNote}</span>
          </div>
        </div>
      </div>

      {loading && (
        <div className="flex items-center justify-center py-8">
          <Spinner className="size-6" />
        </div>
      )}

      {/* Agent List */}
      <div className="rounded-lg border bg-card divide-y">
        {agents.map((agent) => {
          const Icon = ROLE_ICONS[agent.id] || BotIcon;
          return (
            <div key={agent.id} className="px-4 py-3">
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
              <p className="line-clamp-2 text-xs text-muted-foreground mb-2">{agent.description}</p>
              <div className="flex gap-4 text-xs text-muted-foreground">
                <span>{t.agents.temperature}: {agent.temperature}</span>
                <span>{t.agents.maxTokens}: {agent.maxTokens}</span>
              </div>
            </div>
          );
        })}
      </div>

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
    </div>
  );
}
