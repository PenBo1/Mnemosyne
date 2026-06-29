import { useState, useEffect } from "react";
import { useI18n } from "@/shared/i18n";
import type { KanbanTask, CreateKanbanTaskRequest, UpdateKanbanTaskRequest, KanbanTaskStatus, KanbanPriority } from "@/shared/types";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

interface KanbanTaskDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  task: KanbanTask | null;
  onSubmit: (req: CreateKanbanTaskRequest | UpdateKanbanTaskRequest) => Promise<void>;
  novelId?: string;
}

const STATUS_KEYS: KanbanTaskStatus[] = [
  "plan",
  "compose",
  "write",
  "audit",
  "revise",
  "done",
  "cancelled",
];

const PRIORITY_KEYS: KanbanPriority[] = ["low", "medium", "high", "urgent"];

export function KanbanTaskDialog({
  open,
  onOpenChange,
  task,
  onSubmit,
  novelId: _novelId,
}: KanbanTaskDialogProps) {
  const { t } = useI18n();
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [status, setStatus] = useState<KanbanTaskStatus>("plan");
  const [priority, setPriority] = useState<KanbanPriority>("medium");
  const [agent, setAgent] = useState("");
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (task) {
      setTitle(task.title);
      setDescription(task.description);
      setStatus(task.status);
      setPriority(task.priority);
      setAgent(task.assigned_agent ?? "");
    } else {
      setTitle("");
      setDescription("");
      setStatus("plan");
      setPriority("medium");
      setAgent("");
    }
  }, [task, open]);

  const handleSubmit = async () => {
    if (!title.trim()) return;
    setSubmitting(true);
    try {
      if (task) {
        await onSubmit({
          title: title.trim(),
          description: description.trim(),
          status,
          priority,
          assigned_agent: agent || undefined,
        } as UpdateKanbanTaskRequest);
      } else {
        await onSubmit({
          title: title.trim(),
          description: description.trim(),
          status,
          priority,
          assigned_agent: agent || undefined,
        } as CreateKanbanTaskRequest);
      }
      onOpenChange(false);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>
            {task ? t.kanban.editTask : t.kanban.newTask}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          <div className="space-y-2">
            <Label htmlFor="title">{t.kanban.fields.title}</Label>
            <Input
              id="title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder={t.kanban.fields.title}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="description">{t.kanban.fields.description}</Label>
            <Textarea
              id="description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t.kanban.fields.description}
              rows={3}
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>{t.kanban.fields.status}</Label>
              <Select value={status} onValueChange={(v) => setStatus(v as KanbanTaskStatus)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {STATUS_KEYS.map((s) => (
                    <SelectItem key={s} value={s}>
                      {t.kanban.columns[s]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label>{t.kanban.fields.priority}</Label>
              <Select value={priority} onValueChange={(v) => setPriority(v as KanbanPriority)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {PRIORITY_KEYS.map((p) => (
                    <SelectItem key={p} value={p}>
                      {t.kanban.priority[p]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="agent">{t.kanban.fields.agent}</Label>
            <Input
              id="agent"
              value={agent}
              onChange={(e) => setAgent(e.target.value)}
              placeholder={t.kanban.fields.agentPlaceholder}
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t.common.cancel}
          </Button>
          <Button onClick={handleSubmit} disabled={!title.trim() || submitting}>
            {submitting
              ? t.kanban.saving
              : task
                ? t.kanban.saveChanges
                : t.kanban.createTask}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
