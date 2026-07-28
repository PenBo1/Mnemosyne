/**
 * ═══════════════════════════════════════════════════════════════════════════
 * CharactersPage - 角色管理页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useMemo, useState } from "react";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { useI18n } from "@/locales/i18n";
import { parseTags, cn } from "@/lib/utils";
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
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { SearchInput } from "@/components/shared/search-input";
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
  UsersIcon,
  PlusIcon,
  Trash2Icon,
  LayoutGridIcon,
  ListIcon,
} from "lucide-react";
import { useCharacters } from "@/features/story/hooks";
import type { Character } from "@/features/story/types";

export function CharactersPage() {
  const { t } = useI18n();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const { characters, loading, create, update, remove } = useCharacters(activeWorkspaceId);
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Character | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [viewMode, setViewMode] = useState<"grid" | "table">("grid");

  const [formName, setFormName] = useState("");
  const [formRole, setFormRole] = useState("");
  const [formAge, setFormAge] = useState("");
  const [formGender, setFormGender] = useState("");
  const [formAppearance, setFormAppearance] = useState("");
  const [formPersonality, setFormPersonality] = useState("");
  const [formBackstory, setFormBackstory] = useState("");
  const [formMotivation, setFormMotivation] = useState("");
  const [formFears, setFormFears] = useState("");
  const [formSkills, setFormSkills] = useState("");
  const [formDescription, setFormDescription] = useState("");
  const [formTraits, setFormTraits] = useState("");

  const filtered = useMemo(() => {
    const q = search.toLowerCase();
    return characters.filter((c) => c.name.toLowerCase().includes(q));
  }, [characters, search]);

  const resetForm = () => {
    setFormName(""); setFormRole(""); setFormAge(""); setFormGender("");
    setFormAppearance(""); setFormPersonality(""); setFormBackstory("");
    setFormMotivation(""); setFormFears(""); setFormSkills("");
    setFormDescription(""); setFormTraits("");
  };

  const openCreate = () => {
    resetForm();
    setIsEditing(false);
    setDialogOpen(true);
  };

  const openEdit = (c: Character) => {
    setFormName(c.name); setFormRole(c.role); setFormAge(c.age);
    setFormGender(c.gender); setFormAppearance(c.appearance);
    setFormPersonality(c.personality); setFormBackstory(c.backstory);
    setFormMotivation(c.motivation); setFormFears(c.fears);
    setFormSkills(c.skills); setFormDescription(c.description);
    setFormTraits(c.traits.join(", "));
    setIsEditing(true);
    setSelected(c);
    setDialogOpen(true);
  };

  const handleSave = async () => {
    if (!formName.trim()) return;
    const traits = parseTags(formTraits);

    if (isEditing && selected) {
      await update({
        id: selected.id, name: formName, role: formRole, age: formAge,
        gender: formGender, appearance: formAppearance, personality: formPersonality,
        backstory: formBackstory, motivation: formMotivation, fears: formFears,
        skills: formSkills, description: formDescription, traits,
      });
    } else {
      await create({
        name: formName, role: formRole, age: formAge,
        gender: formGender, appearance: formAppearance, personality: formPersonality,
        backstory: formBackstory, motivation: formMotivation, fears: formFears,
        skills: formSkills, description: formDescription, traits,
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
            <UsersIcon />
            {t.characters.title}
          </PageTitle>
          <PageDescription>{t.characters.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button onClick={openCreate}>
            <PlusIcon data-icon="inline-start" />
            {t.characters.create}
          </Button>
        </PageActions>
      </PageHeader>

      <div className="flex items-center gap-4">
        <SearchInput
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t.common.search}
          className="flex-1"
        />
        <Tabs value={viewMode} onValueChange={(v) => setViewMode(v as "grid" | "table")}>
          <TabsList>
            <TabsTrigger value="grid">
              <LayoutGridIcon className="size-3" />
              {t.characters.gridView}
            </TabsTrigger>
            <TabsTrigger value="table">
              <ListIcon className="size-3" />
              {t.characters.graphView}
            </TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      {loading ? (
        <LoadingState label={t.common.loading} />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<UsersIcon className="size-6" />} title={t.characters.empty} />
      ) : viewMode === "grid" ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {filtered.map((c) => (
            <Card
              key={c.id}
              onClick={() => openEdit(c)}
              className={cn(
                "cursor-pointer transition-colors group",
                selected?.id === c.id ? "ring-[var(--border-brand-l1)] bg-[var(--bg-overlay-l3)]" : "hover:bg-[var(--bg-overlay-l2)]"
              )}
            >
              <CardHeader className="pb-2">
                <div className="flex items-center justify-between">
                  <CardTitle className="text-base">{c.name}</CardTitle>
                  <Button variant="ghost" size="icon-sm" onClick={(e) => { e.stopPropagation(); handleDelete(c.id); }} className="opacity-0 group-hover:opacity-100 text-destructive">
                    <Trash2Icon />
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="flex flex-col gap-2">
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground">{c.role}</span>
                  {c.age && <span className="text-xs text-muted-foreground">· {c.age}</span>}
                </div>
                {c.description && (
                  <p className="text-xs text-muted-foreground line-clamp-2">{c.description}</p>
                )}
                {c.traits.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    {c.traits.slice(0, 3).map((trait) => (
                      <Badge key={trait} variant="outline">{trait}</Badge>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <Card>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t.characters.name}</TableHead>
                <TableHead>{t.characters.role}</TableHead>
                <TableHead>{t.characters.age}</TableHead>
                <TableHead>{t.characters.gender}</TableHead>
                <TableHead>{t.characters.traits}</TableHead>
                <TableHead className="w-8" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.map((c) => (
                <TableRow
                  key={c.id}
                  onClick={() => openEdit(c)}
                  className="cursor-pointer"
                >
                  <TableCell className="font-medium">{c.name}</TableCell>
                  <TableCell>{c.role}</TableCell>
                  <TableCell>{c.age || "\u2014"}</TableCell>
                  <TableCell>{c.gender || "\u2014"}</TableCell>
                  <TableCell>
                    <div className="flex flex-wrap gap-1">
                      {c.traits.slice(0, 2).map((trait) => (
                        <Badge key={trait} variant="outline" className="text-xs">{trait}</Badge>
                      ))}
                    </div>
                  </TableCell>
                  <TableCell>
                    <Button variant="ghost" size="icon-sm" onClick={(e) => { e.stopPropagation(); handleDelete(c.id); }} className="text-destructive">
                      <Trash2Icon />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl max-h-[80vh]">
          <DialogHeader>
            <DialogTitle>{isEditing ? t.characters.edit : t.characters.create}</DialogTitle>
          </DialogHeader>
          <ScrollArea className="max-h-[70vh]">
            <FieldGroup>
              <div className="grid grid-cols-2 gap-4">
                <Field>
                  <FieldLabel>{t.characters.name}</FieldLabel>
                  <Input value={formName} onChange={(e) => setFormName(e.target.value)} placeholder={t.characters.namePlaceholder} />
                </Field>
                <Field>
                  <FieldLabel>{t.characters.role}</FieldLabel>
                  <Input value={formRole} onChange={(e) => setFormRole(e.target.value)} placeholder={t.characters.rolePlaceholder} />
                </Field>
                <Field>
                  <FieldLabel>{t.characters.age}</FieldLabel>
                  <Input value={formAge} onChange={(e) => setFormAge(e.target.value)} />
                </Field>
                <Field>
                  <FieldLabel>{t.characters.gender}</FieldLabel>
                  <Input value={formGender} onChange={(e) => setFormGender(e.target.value)} />
                </Field>
              </div>
              <Field>
                <FieldLabel>{t.characters.description_label}</FieldLabel>
                <Textarea value={formDescription} onChange={(e) => setFormDescription(e.target.value)} placeholder={t.characters.descriptionPlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.characters.appearance}</FieldLabel>
                <Textarea value={formAppearance} onChange={(e) => setFormAppearance(e.target.value)} placeholder={t.characters.appearancePlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.characters.personality}</FieldLabel>
                <Textarea value={formPersonality} onChange={(e) => setFormPersonality(e.target.value)} placeholder={t.characters.personalityPlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.characters.backstory}</FieldLabel>
                <Textarea value={formBackstory} onChange={(e) => setFormBackstory(e.target.value)} placeholder={t.characters.backstoryPlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.characters.motivation}</FieldLabel>
                <Textarea value={formMotivation} onChange={(e) => setFormMotivation(e.target.value)} placeholder={t.characters.motivationPlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.characters.fears}</FieldLabel>
                <Textarea value={formFears} onChange={(e) => setFormFears(e.target.value)} placeholder={t.characters.fearsPlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.characters.skills}</FieldLabel>
                <Textarea value={formSkills} onChange={(e) => setFormSkills(e.target.value)} placeholder={t.characters.skillsPlaceholder} />
              </Field>
              <Field>
                <FieldLabel>{t.characters.traits}</FieldLabel>
                <Input value={formTraits} onChange={(e) => setFormTraits(e.target.value)} placeholder={t.characters.traitsPlaceholder} />
              </Field>
            </FieldGroup>
          </ScrollArea>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>{t.common.cancel}</Button>
            <Button onClick={handleSave} disabled={!formName.trim()}>
              {isEditing ? t.characters.update : t.characters.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}