import { useMemo, useState } from "react";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { useI18n } from "@/locales/i18n";
import { parseTags } from "@/lib/utils";
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
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Timeline,
  TimelineItem,
  TimelineSeparator,
  TimelineDot,
  TimelineConnector,
  TimelineContent,
  TimelineHeader,
  TimelineTitle,
  TimelineDescription,
  TimelineDate,
} from "@/components/ui/timeline";
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
  ClockIcon,
  PlusIcon,
  Trash2Icon,
  NetworkIcon,
  ListIcon,
} from "lucide-react";
import { useTimelineEvents } from "@/features/story/hooks";
import type { TimelineEvent, TimelineEventType } from "@/features/story/types";

export function TimelinePage() {
  const { t } = useI18n();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
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
    const tags = parseTags(formTags);

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

  const sorted = useMemo(
    () => [...events].sort((a, b) => a.sort_order - b.sort_order),
    [events],
  );

  const getDotVariant = (eventType: TimelineEventType): "default" | "primary" | "secondary" | "destructive" | "muted" => {
    switch (eventType) {
      case "turning_point": return "destructive";
      case "milestone": return "muted";
      default: return "default";
    }
  };

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <ClockIcon />
            {t.timeline.title}
          </PageTitle>
          <PageDescription>{t.timeline.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button onClick={openCreate}>
            <PlusIcon data-icon="inline-start" />
            {t.timeline.create}
          </Button>
        </PageActions>
      </PageHeader>

      <Tabs value={view} onValueChange={(v) => setView(v as "list" | "chart")} className="flex-1">
        <TabsList>
          <TabsTrigger value="list"><ListIcon className="size-3" /> {t.timeline.listView}</TabsTrigger>
          <TabsTrigger value="chart"><NetworkIcon className="size-3" /> {t.timeline.chartView}</TabsTrigger>
        </TabsList>

        <TabsContent value="list" className="flex-1">
          {loading ? (
            <LoadingState label={t.common.loading} />
          ) : events.length === 0 ? (
            <EmptyState icon={<ClockIcon />} title={t.timeline.empty} />
          ) : (
            <Timeline>
              {sorted.map((ev, idx) => (
                <TimelineItem key={ev.id} onClick={() => openEdit(ev)} className="cursor-pointer group">
                  <TimelineSeparator>
                    <TimelineDot variant={getDotVariant(ev.event_type)} />
                    {idx < sorted.length - 1 && <TimelineConnector />}
                  </TimelineSeparator>
                  <TimelineContent>
                    <TimelineHeader>
                      <TimelineTitle>{ev.title}</TimelineTitle>
                      <TimelineDate>{ev.event_date || "\u2014"}</TimelineDate>
                    </TimelineHeader>
                    <TimelineDescription>
                      {ev.description || "\u2014"}
                    </TimelineDescription>
                    <div className="flex items-center gap-2 mt-1">
                      <Badge variant="outline" className="text-xs uppercase">{t.timeline.types[ev.event_type]}</Badge>
                      {ev.chapter_number != null && (
                        <Badge variant="secondary" className="text-xs">{t.plot.chapterNumber} {ev.chapter_number}</Badge>
                      )}
                      {ev.tags.length > 0 && (
                        <div className="flex flex-wrap gap-1">
                          {ev.tags.slice(0, 3).map((tag) => (
                            <Badge key={tag} variant="outline" className="text-xs">{tag}</Badge>
                          ))}
                        </div>
                      )}
                      <Button variant="ghost" size="icon-sm" onClick={(e) => { e.stopPropagation(); handleDelete(ev.id); }} className="opacity-0 group-hover:opacity-100 text-destructive">
                        <Trash2Icon />
                      </Button>
                    </div>
                  </TimelineContent>
                </TimelineItem>
              ))}
            </Timeline>
          )}
        </TabsContent>

        <TabsContent value="chart" className="flex-1">
          {loading ? (
            <LoadingState label={t.common.loading} />
          ) : events.length === 0 ? (
            <EmptyState icon={<NetworkIcon />} title={t.timeline.empty} />
          ) : (
            <Card>
              <CardContent className="p-4">
                <div className="text-sm text-muted-foreground text-center">
                  {t.timeline.chartView}
                </div>
              </CardContent>
            </Card>
          )}
        </TabsContent>
      </Tabs>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh]">
          <DialogHeader>
            <DialogTitle>{isEditing ? t.timeline.edit : t.timeline.create}</DialogTitle>
          </DialogHeader>
          <ScrollArea className="max-h-[70vh]">
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
          </ScrollArea>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>{t.common.cancel}</Button>
            <Button onClick={handleSave} disabled={!formTitle.trim()}>
              {isEditing ? t.timeline.update : t.timeline.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}