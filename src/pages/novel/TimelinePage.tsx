import { useState } from "react";
import { useWorkspaceStore } from "@/stores/workspace";
import { useI18n } from "@/lib/i18n";
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
import {
  ClockIcon,
  PlusIcon,
  Trash2Icon,
  NetworkIcon,
  ListIcon,
} from "lucide-react";
import { useTimelineEvents } from "@/hooks/useTimelineEvents";
import type { TimelineEvent, TimelineEventType } from "@/types";

export function TimelinePage() {
  const { t } = useI18n();
  const { activeWorkspaceId } = useWorkspaceStore();
  const { events, loading, create, update, remove } = useTimelineEvents(activeWorkspaceId);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [selected, setSelected] = useState<TimelineEvent | null>(null);
  const [view, setView] = useState<"list" | "chart">("list");

  const [formTitle, setFormTitle] = useState("");
  const [formDescription, setFormDescription] = useState("");
  const [formEventDate, setFormEventDate] = useState("");
  const [formEventType, setFormEventType] = useState<TimelineEventType>("event");
  const [formChapterNumber, setFormChapterNumber] = useState("");
  const [formTags, setFormTags] = useState("");

  const resetForm = () => {
    setFormTitle(""); setFormDescription(""); setFormEventDate("");
    setFormEventType("event"); setFormChapterNumber(""); setFormTags("");
  };

  const openCreate = () => { resetForm(); setIsEditing(false); setDialogOpen(true); };

  const openEdit = (ev: TimelineEvent) => {
    setFormTitle(ev.title); setFormDescription(ev.description);
    setFormEventDate(ev.event_date); setFormEventType(ev.event_type);
    setFormChapterNumber(ev.chapter_number?.toString() || "");
    setFormTags(ev.tags.join(", "));
    setIsEditing(true); setSelected(ev); setDialogOpen(true);
  };

  const handleSave = async () => {
    if (!formTitle.trim()) return;
    const chapterNum = formChapterNumber ? parseInt(formChapterNumber) : null;
    const tags = formTags.split(",").map((s) => s.trim()).filter(Boolean);

    if (isEditing && selected) {
      await update({
        id: selected.id, title: formTitle, description: formDescription,
        event_date: formEventDate, event_type: formEventType,
        chapter_number: chapterNum, tags,
      });
    } else {
      await create({
        title: formTitle, description: formDescription,
        event_date: formEventDate, event_type: formEventType,
        chapter_number: chapterNum, tags, sort_order: events.length,
        character_ids: [],
      });
    }
    setDialogOpen(false);
  };

  const handleDelete = async (id: string) => {
    await remove(id);
    if (selected?.id === id) setSelected(null);
  };

  const sorted = [...events].sort((a, b) => a.sort_order - b.sort_order);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <ClockIcon />
            {t.timeline.title}
          </h1>
          <p className="text-sm text-muted-foreground">{t.timeline.description}</p>
        </div>
        <Button onClick={openCreate}>
          <PlusIcon data-icon="inline-start" />
          {t.timeline.create}
        </Button>
      </div>

      <Tabs value={view} onValueChange={(v) => setView(v as "list" | "chart")}>
        <TabsList>
          <TabsTrigger value="list"><ListIcon className="size-3" /> {t.timeline.listView}</TabsTrigger>
          <TabsTrigger value="chart"><NetworkIcon className="size-3" /> {t.timeline.chartView}</TabsTrigger>
        </TabsList>
      </Tabs>

      {loading ? (
        <div className="text-center py-8 text-muted-foreground">{t.common.loading}</div>
      ) : events.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <ClockIcon className="size-12 mx-auto mb-4 opacity-50" />
          <p>{t.timeline.empty}</p>
        </div>
      ) : (
        <div className="relative">
          <div className="absolute left-[72px] top-0 bottom-0 w-px bg-border" />
          <div className="space-y-4">
            {sorted.map((ev) => (
              <div
                key={ev.id}
                className={`flex items-start gap-4 cursor-pointer group ${selected?.id === ev.id ? "" : ""}`}
                onClick={() => openEdit(ev)}
              >
                <div className="w-16 text-right shrink-0 pt-1">
                  <span className="text-xs text-muted-foreground">{ev.event_date || "\u2014"}</span>
                </div>
                <div className="relative z-10 mt-2">
                  <div className={`size-3 rounded-full border-2 ${
                    ev.event_type === "turning_point" ? "bg-red-500 border-red-500" :
                    ev.event_type === "milestone" ? "bg-yellow-500 border-yellow-500" :
                    "bg-primary border-primary"
                  }`} />
                </div>
                <div className={`flex-1 rounded-lg border p-3 transition-colors ${
                  selected?.id === ev.id ? "border-primary bg-primary/5" : "hover:bg-muted"
                }`}>
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <span className="text-[10px] uppercase font-medium text-muted-foreground">
                        {t.timeline.types[ev.event_type]}
                      </span>
                      <span className="font-medium">{ev.title}</span>
                    </div>
                    <button
                      onClick={(e) => { e.stopPropagation(); handleDelete(ev.id); }}
                      className="opacity-0 group-hover:opacity-100 hover:text-destructive"
                    >
                      <Trash2Icon className="size-3" />
                    </button>
                  </div>
                  {ev.description && (
                    <p className="text-xs text-muted-foreground mt-1">{ev.description}</p>
                  )}
                  {ev.tags.length > 0 && (
                    <div className="flex flex-wrap gap-1 mt-2">
                      {ev.tags.map((tag) => (
                        <span key={tag} className="text-[10px] bg-muted px-1.5 py-0.5 rounded">{tag}</span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{isEditing ? t.timeline.edit : t.timeline.create}</DialogTitle>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.timeline.title_label}</FieldLabel>
              <Input value={formTitle} onChange={(e) => setFormTitle(e.target.value)} placeholder={t.timeline.titlePlaceholder} />
            </Field>
            <div className="grid grid-cols-3 gap-4">
              <Field>
                <FieldLabel>{t.timeline.eventType}</FieldLabel>
                <Select value={formEventType} onValueChange={(v) => setFormEventType(v as TimelineEventType)}>
                  <SelectTrigger><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="event">{t.timeline.types.event}</SelectItem>
                    <SelectItem value="milestone">{t.timeline.types.milestone}</SelectItem>
                    <SelectItem value="turning_point">{t.timeline.types.turning_point}</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field>
                <FieldLabel>{t.timeline.eventDate}</FieldLabel>
                <Input value={formEventDate} onChange={(e) => setFormEventDate(e.target.value)} placeholder={t.timeline.eventDatePlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.timeline.chapterNumber}</FieldLabel>
                <Input type="number" value={formChapterNumber} onChange={(e) => setFormChapterNumber(e.target.value)} />
              </Field>
            </div>
            <Field>
              <FieldLabel>{t.timeline.description_label}</FieldLabel>
              <Textarea value={formDescription} onChange={(e) => setFormDescription(e.target.value)} placeholder={t.timeline.descriptionPlaceholder} />
            </Field>
            <Field>
              <FieldLabel>{t.timeline.tags}</FieldLabel>
              <Input value={formTags} onChange={(e) => setFormTags(e.target.value)} placeholder={t.timeline.tagsPlaceholder} />
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>{t.common.cancel}</Button>
            <Button onClick={handleSave} disabled={!formTitle.trim()}>
              {isEditing ? t.timeline.update : t.timeline.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
