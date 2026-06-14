import { useState, useEffect, useCallback } from "react";
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
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  GlobeIcon,
  PlusIcon,
  Trash2Icon,
  SearchIcon,
} from "lucide-react";
import { ipc } from "@/lib/ipc";
import type { WorldSetting, WorldCategory } from "@/types";

const CATEGORIES: WorldCategory[] = [
  "location", "faction", "species", "culture",
  "history", "magic_system", "language", "architecture",
];

export function WorldbuildingPage() {
  const { t } = useI18n();
  const { activeWorkspaceId } = useWorkspaceStore();
  const [items, setItems] = useState<WorldSetting[]>([]);
  const [loading, setLoading] = useState(true);
  const [category, setCategory] = useState<WorldCategory>("location");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<WorldSetting | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);

  const [formName, setFormName] = useState("");
  const [formDescription, setFormDescription] = useState("");
  const [formContent, setFormContent] = useState("");
  const [formTags, setFormTags] = useState("");

  const loadItems = useCallback(async () => {
    if (!activeWorkspaceId) return;
    setLoading(true);
    try {
      const novels = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
      const novel = novels.find((n) => n.workspace_id === activeWorkspaceId);
      if (!novel) { setItems([]); return; }
      const data = await ipc<WorldSetting[]>("world_setting_list", { novelId: novel.id });
      setItems(data);
    } catch {
      setItems([]);
    } finally {
      setLoading(false);
    }
  }, [activeWorkspaceId]);

  useEffect(() => { loadItems(); }, [loadItems]);

  const filtered = items.filter(
    (i) => i.category === category && i.name.toLowerCase().includes(search.toLowerCase())
  );

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
    if (!activeWorkspaceId || !formName.trim()) return;
    const novels = await ipc<{ id: string; workspace_id: string }[]>("list_novels");
    const novel = novels.find((n) => n.workspace_id === activeWorkspaceId);
    if (!novel) return;

    const tags = formTags.split(",").map((s) => s.trim()).filter(Boolean);

    if (isEditing && selected) {
      await ipc("world_setting_update", {
        id: selected.id, name: formName, description: formDescription,
        content: formContent, tags,
      });
    } else {
      await ipc("world_setting_create", {
        novelId: novel.id, category, name: formName,
        description: formDescription, content: formContent, tags,
      });
    }
    setDialogOpen(false);
    await loadItems();
  };

  const handleDelete = async (id: string) => {
    await ipc("world_setting_delete", { id });
    if (selected?.id === id) setSelected(null);
    await loadItems();
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <GlobeIcon />
            {t.worldbuilding.title}
          </h1>
          <p className="text-sm text-muted-foreground">{t.worldbuilding.description}</p>
        </div>
        <Button onClick={openCreate}>
          <PlusIcon data-icon="inline-start" />
          {t.worldbuilding.create}
        </Button>
      </div>

      <Tabs value={category} onValueChange={(v) => setCategory(v as WorldCategory)}>
        <TabsList className="flex-wrap h-auto">
          {CATEGORIES.map((cat) => (
            <TabsTrigger key={cat} value={cat} className="text-xs">
              {t.worldbuilding.categories[cat]}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>

      <div className="relative">
        <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
        <Input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t.common.search}
          className="pl-9"
        />
      </div>

      {loading ? (
        <div className="text-center py-8 text-muted-foreground">{t.common.loading}</div>
      ) : filtered.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <GlobeIcon className="size-12 mx-auto mb-4 opacity-50" />
          <p>{t.worldbuilding.empty}</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {filtered.map((item) => (
            <button
              key={item.id}
              onClick={() => openEdit(item)}
              className={`text-left rounded-lg border p-4 transition-colors ${
                selected?.id === item.id ? "border-primary bg-primary/5" : "hover:bg-muted"
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-medium">{item.name}</span>
                <button
                  onClick={(e) => { e.stopPropagation(); handleDelete(item.id); }}
                  className="opacity-0 group-hover:opacity-100 hover:text-destructive"
                >
                  <Trash2Icon className="size-3" />
                </button>
              </div>
              {item.description && (
                <p className="text-xs text-muted-foreground mt-1 line-clamp-2">{item.description}</p>
              )}
              {item.tags.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-2">
                  {item.tags.slice(0, 3).map((tag) => (
                    <span key={tag} className="text-[10px] bg-muted px-1.5 py-0.5 rounded">{tag}</span>
                  ))}
                </div>
              )}
            </button>
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
    </div>
  );
}
