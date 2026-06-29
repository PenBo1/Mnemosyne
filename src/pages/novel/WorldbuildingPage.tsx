import { useMemo, useState } from "react";
import { useWorkspaceStore } from "@/stores/workspace";
import { useI18n } from "@/shared/i18n";
import { parseTags } from "@/shared/utils";
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
  GlobeIcon,
  PlusIcon,
  Trash2Icon,
  SearchIcon,
  NetworkIcon,
  LayoutGridIcon,
} from "lucide-react";
import { useWorldSettings } from "@/features/story/hooks";
import type { WorldSetting, WorldCategory } from "@/shared/types";

const CATEGORIES: WorldCategory[] = [
  "location", "faction", "species", "culture",
  "history", "magic_system", "language", "architecture",
];

export function WorldbuildingPage() {
  const { t } = useI18n();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const { items, loading, create, update, remove } = useWorldSettings(activeWorkspaceId);
  const [category, setCategory] = useState<WorldCategory>("location");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<WorldSetting | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [view, setView] = useState<"grid" | "network">("grid");

  const [formName, setFormName] = useState("");
  const [formDescription, setFormDescription] = useState("");
  const [formContent, setFormContent] = useState("");
  const [formTags, setFormTags] = useState("");

  const filtered = useMemo(() => {
    const q = search.toLowerCase();
    return items.filter(
      (i) => i.category === category && i.name.toLowerCase().includes(q)
    );
  }, [items, category, search]);

  const resetForm = () => {
    setFormName(""); setFormDescription(""); setFormContent(""); setFormTags("");
  };

  const openCreate = () => {
    resetForm();
    setIsEditing(false);
    setDialogOpen(true);
  };

  const openEdit = (item: WorldSetting) => {
    setFormName(item.name); setFormDescription(item.description);
    setFormContent(item.content); setFormTags(item.tags.join(", "));
    setIsEditing(true);
    setSelected(item);
    setDialogOpen(true);
  };

  const handleSave = async () => {
    if (!formName.trim()) return;
    const tags = parseTags(formTags);

    if (isEditing && selected) {
      await update({
        id: selected.id, name: formName, description: formDescription,
        content: formContent, tags,
      });
    } else {
      await create({
        category, name: formName,
        description: formDescription, content: formContent, tags,
      });
    }
    setDialogOpen(false);
  };

  const handleDelete = async (id: string) => {
    await remove(id);
    if (selected?.id === id) setSelected(null);
  };

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <GlobeIcon />
            {t.worldbuilding.title}
          </PageTitle>
          <PageDescription>{t.worldbuilding.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button onClick={openCreate}>
            <PlusIcon data-icon="inline-start" />
            {t.worldbuilding.create}
          </Button>
        </PageActions>
      </PageHeader>

      <Tabs value={category} onValueChange={(v) => setCategory(v as WorldCategory)}>
        <TabsList className="flex-wrap h-auto">
          {CATEGORIES.map((cat) => (
            <TabsTrigger key={cat} value={cat} className="text-xs">
              {t.worldbuilding.categories[cat]}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>

      <div className="flex items-center justify-between">
        <div className="relative flex-1 max-w-sm">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t.common.search}
            className="pl-9"
          />
        </div>
        <Tabs value={view} onValueChange={(v) => setView(v as "grid" | "network")}>
          <TabsList>
            <TabsTrigger value="grid"><LayoutGridIcon className="size-3" /> {t.worldbuilding.gridView}</TabsTrigger>
            <TabsTrigger value="network"><NetworkIcon className="size-3" /> {t.worldbuilding.networkView}</TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      {loading ? (
        <LoadingState label={t.common.loading} />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<GlobeIcon />} title={t.worldbuilding.empty} />
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {filtered.map((item) => (
            <div
              key={item.id}
              role="button"
              tabIndex={0}
              onClick={() => openEdit(item)}
              onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openEdit(item); } }}
              className={`flex flex-col gap-2 text-left rounded-[var(--radius-6)] border p-4 transition-colors group cursor-pointer ${
                selected?.id === item.id ? "border-primary bg-primary/5" : "hover:bg-[var(--bg-overlay-l2)]"
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-medium">{item.name}</span>
                <button
                  onClick={(e) => { e.stopPropagation(); handleDelete(item.id); }}
                  className="opacity-0 group-hover:opacity-100 transition-opacity hover:text-destructive"
                >
                  <Trash2Icon className="size-3" />
                </button>
              </div>
              {item.description && (
                <p className="text-xs text-muted-foreground line-clamp-2">{item.description}</p>
              )}
              {item.tags.length > 0 && (
                <div className="flex flex-wrap gap-1">
                  {item.tags.slice(0, 3).map((tag) => (
                    <Badge key={tag} variant="outline">{tag}</Badge>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{isEditing ? t.worldbuilding.edit : t.worldbuilding.create}</DialogTitle>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.worldbuilding.name}</FieldLabel>
              <Input value={formName} onChange={(e) => setFormName(e.target.value)} placeholder={t.worldbuilding.namePlaceholder} />
            </Field>
            <Field>
              <FieldLabel>{t.worldbuilding.description_label}</FieldLabel>
              <Textarea value={formDescription} onChange={(e) => setFormDescription(e.target.value)} placeholder={t.worldbuilding.descriptionPlaceholder} />
            </Field>
            <Field>
              <FieldLabel>{t.worldbuilding.content}</FieldLabel>
              <Textarea value={formContent} onChange={(e) => setFormContent(e.target.value)} placeholder={t.worldbuilding.contentPlaceholder} className="min-h-[200px]" />
            </Field>
            <Field>
              <FieldLabel>{t.worldbuilding.tags}</FieldLabel>
              <Input value={formTags} onChange={(e) => setFormTags(e.target.value)} placeholder={t.worldbuilding.tagsPlaceholder} />
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>{t.common.cancel}</Button>
            <Button onClick={handleSave} disabled={!formName.trim()}>
              {isEditing ? t.worldbuilding.update : t.worldbuilding.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}
