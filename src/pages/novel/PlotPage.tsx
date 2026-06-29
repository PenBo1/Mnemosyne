import { useMemo, useState } from "react";
import { useWorkspaceStore } from "@/stores/workspace";
import { useI18n } from "@/shared/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState, EmptyState } from "@/components/shared/state";
import {
  GitBranchIcon,
  PlusIcon,
  Trash2Icon,
  GripVerticalIcon,
  ClockIcon,
  TreePineIcon,
} from "lucide-react";
import { usePlotPoints } from "@/features/story/hooks";
import type { PlotPoint, PlotPointType } from "@/shared/types";

export function PlotPage() {
  const { t } = useI18n();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const { points, loading, create, update, remove } = usePlotPoints(activeWorkspaceId);
  const [view, setView] = useState<"outline" | "timeline" | "tree">("outline");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [selected, setSelected] = useState<PlotPoint | null>(null);

  const [formTitle, setFormTitle] = useState("");
  const [formDescription, setFormDescription] = useState("");
  const [formType, setFormType] = useState<PlotPointType>("scene");
  const [formStatus, setFormStatus] = useState("planned");
  const [formChapterNumber, setFormChapterNumber] = useState("");
  const [formGoals, setFormGoals] = useState("");
  const [formConflicts, setFormConflicts] = useState("");
  const [formOutcome, setFormOutcome] = useState("");

  const resetForm = () => {
    setFormTitle(""); setFormDescription(""); setFormType("scene");
    setFormStatus("planned"); setFormChapterNumber("");
    setFormGoals(""); setFormConflicts(""); setFormOutcome("");
  };

  const openCreate = () => { resetForm(); setIsEditing(false); setDialogOpen(true); };

  const openEdit = (p: PlotPoint) => {
    setFormTitle(p.title); setFormDescription(p.description);
    setFormType(p.type); setFormStatus(p.status);
    setFormChapterNumber(p.chapter_number?.toString() || "");
    setFormGoals(p.goals); setFormConflicts(p.conflicts); setFormOutcome(p.outcome);
    setIsEditing(true); setSelected(p); setDialogOpen(true);
  };

  const handleSave = async () => {
    if (!formTitle.trim()) return;
    const chapterNum = formChapterNumber ? parseInt(formChapterNumber) : null;

    if (isEditing && selected) {
      await update({
        id: selected.id, title: formTitle, description: formDescription,
        type: formType, status: formStatus, chapter_number: chapterNum,
        goals: formGoals, conflicts: formConflicts, outcome: formOutcome,
      });
    } else {
      await create({
        type: formType, title: formTitle,
        description: formDescription, status: formStatus,
        chapter_number: chapterNum, goals: formGoals,
        conflicts: formConflicts, outcome: formOutcome,
        sort_order: points.length,
      });
    }
    setDialogOpen(false);
  };

  const handleDelete = async (id: string) => {
    await remove(id);
    if (selected?.id === id) setSelected(null);
  };

  const outlineItems = useMemo(
    () => points.filter((p) => p.type === "act" || p.type === "chapter" || p.type === "scene"),
    [points],
  );
  const sortedByChapter = useMemo(
    () => [...points].sort((a, b) => (a.chapter_number || 0) - (b.chapter_number || 0)),
    [points],
  );

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <GitBranchIcon />
            {t.plot.title}
          </PageTitle>
          <PageDescription>{t.plot.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button onClick={openCreate}>
            <PlusIcon data-icon="inline-start" />
            {t.plot.create}
          </Button>
        </PageActions>
      </PageHeader>

      <Tabs value={view} onValueChange={(v) => setView(v as "outline" | "timeline" | "tree")}>
        <TabsList>
          <TabsTrigger value="outline"><GitBranchIcon className="size-3" /> {t.plot.outlineView}</TabsTrigger>
          <TabsTrigger value="timeline"><ClockIcon className="size-3" /> {t.plot.timelineView}</TabsTrigger>
          <TabsTrigger value="tree"><TreePineIcon className="size-3" /> {t.plot.treeView}</TabsTrigger>
        </TabsList>
      </Tabs>

      {loading ? (
        <LoadingState label={t.common.loading} />
      ) : points.length === 0 ? (
        <EmptyState icon={<GitBranchIcon />} title={t.plot.empty} />
      ) : view === "outline" ? (
        <div className="flex flex-col gap-2">
          {outlineItems.map((p) => (
            <div
              key={p.id}
              className={`flex items-center gap-3 rounded-[var(--radius-6)] border p-3 cursor-pointer transition-colors group ${
                selected?.id === p.id ? "border-primary bg-primary/5" : "hover:bg-[var(--bg-overlay-l2)]"
              }`}
              style={{ paddingLeft: `${(p.type === "act" ? 0 : p.type === "chapter" ? 1 : 2) * 24 + 12}px` }}
              onClick={() => openEdit(p)}
            >
              <GripVerticalIcon className="size-4 text-muted-foreground shrink-0" />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-[10px] uppercase font-medium text-muted-foreground">{t.plot.types[p.type]}</span>
                  <span className="font-medium truncate">{p.title}</span>
                </div>
                {p.description && (
                  <p className="text-xs text-muted-foreground truncate">{p.description}</p>
                )}
              </div>
              <Badge
                variant="outline"
                className={
                  p.status === "completed"
                    ? "border-transparent bg-primary/10 text-primary"
                    : "border-transparent bg-muted text-muted-foreground"
                }
              >
                {t.plot.statuses[p.status as keyof typeof t.plot.statuses]}
              </Badge>
              <button
                onClick={(e) => { e.stopPropagation(); handleDelete(p.id); }}
                className="opacity-0 group-hover:opacity-100 transition-opacity hover:text-destructive shrink-0"
              >
                <Trash2Icon className="size-3" />
              </button>
            </div>
          ))}
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          {sortedByChapter.map((p) => (
            <div
              key={p.id}
              className={`flex items-center gap-3 rounded-[var(--radius-6)] border p-3 cursor-pointer transition-colors group ${
                selected?.id === p.id ? "border-primary bg-primary/5" : "hover:bg-[var(--bg-overlay-l2)]"
              }`}
              onClick={() => openEdit(p)}
            >
              <div className="text-xs text-muted-foreground w-16 text-right shrink-0">
                {p.chapter_number != null ? `${t.plot.chapterNumber} ${p.chapter_number}` : "\u2014"}
              </div>
              <div className="w-px h-8 bg-border shrink-0" />
              <div className="flex-1 min-w-0">
                <span className="font-medium">{p.title}</span>
                {p.description && (
                  <p className="text-xs text-muted-foreground truncate">{p.description}</p>
                )}
              </div>
              <button
                onClick={(e) => { e.stopPropagation(); handleDelete(p.id); }}
                className="opacity-0 group-hover:opacity-100 transition-opacity hover:text-destructive shrink-0"
              >
                <Trash2Icon className="size-3" />
              </button>
            </div>
          ))}
        </div>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{isEditing ? t.plot.edit : t.plot.create}</DialogTitle>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.plot.title_label}</FieldLabel>
              <Input value={formTitle} onChange={(e) => setFormTitle(e.target.value)} placeholder={t.plot.titlePlaceholder} />
            </Field>
            <div className="grid grid-cols-3 gap-4">
              <Field>
                <FieldLabel>{t.plot.type}</FieldLabel>
                <Select value={formType} onValueChange={(v) => setFormType(v as PlotPointType)}>
                  <SelectTrigger><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="act">{t.plot.types.act}</SelectItem>
                    <SelectItem value="chapter">{t.plot.types.chapter}</SelectItem>
                    <SelectItem value="scene">{t.plot.types.scene}</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field>
                <FieldLabel>{t.plot.status}</FieldLabel>
                <Select value={formStatus} onValueChange={setFormStatus}>
                  <SelectTrigger><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="planned">{t.plot.statuses.planned}</SelectItem>
                    <SelectItem value="in_progress">{t.plot.statuses.in_progress}</SelectItem>
                    <SelectItem value="completed">{t.plot.statuses.completed}</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field>
                <FieldLabel>{t.plot.chapterNumber}</FieldLabel>
                <Input type="number" value={formChapterNumber} onChange={(e) => setFormChapterNumber(e.target.value)} />
              </Field>
            </div>
            <Field>
              <FieldLabel>{t.plot.description_label}</FieldLabel>
              <Textarea value={formDescription} onChange={(e) => setFormDescription(e.target.value)} placeholder={t.plot.descriptionPlaceholder} />
            </Field>
            <Field>
              <FieldLabel>{t.plot.goals}</FieldLabel>
              <Textarea value={formGoals} onChange={(e) => setFormGoals(e.target.value)} placeholder={t.plot.goalsPlaceholder} />
            </Field>
            <Field>
              <FieldLabel>{t.plot.conflicts}</FieldLabel>
              <Textarea value={formConflicts} onChange={(e) => setFormConflicts(e.target.value)} placeholder={t.plot.conflictsPlaceholder} />
            </Field>
            <Field>
              <FieldLabel>{t.plot.outcome}</FieldLabel>
              <Textarea value={formOutcome} onChange={(e) => setFormOutcome(e.target.value)} placeholder={t.plot.outcomePlaceholder} />
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>{t.common.cancel}</Button>
            <Button onClick={handleSave} disabled={!formTitle.trim()}>
              {isEditing ? t.plot.update : t.plot.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}
