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
  UsersIcon,
  PlusIcon,
  Trash2Icon,
  SearchIcon,
} from "lucide-react";
import { useCharacters } from "@/hooks/useCharacters";
import type { Character } from "@/types";

export function CharactersPage() {
  const { t } = useI18n();
  const { activeWorkspaceId } = useWorkspaceStore();
  const { characters, loading, create, update, remove } = useCharacters(activeWorkspaceId);
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Character | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);

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

  const filtered = characters.filter((c) =>
    c.name.toLowerCase().includes(search.toLowerCase())
  );

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
    const traits = formTraits.split(",").map((s) => s.trim()).filter(Boolean);

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
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <UsersIcon />
            {t.characters.title}
          </h1>
          <p className="text-sm text-muted-foreground">{t.characters.description}</p>
        </div>
        <Button onClick={openCreate}>
          <PlusIcon data-icon="inline-start" />
          {t.characters.create}
        </Button>
      </div>

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
          <UsersIcon className="size-12 mx-auto mb-4 opacity-50" />
          <p>{t.characters.empty}</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {filtered.map((c) => (
            <button
              key={c.id}
              onClick={() => openEdit(c)}
              className={`text-left rounded-lg border p-4 transition-colors ${
                selected?.id === c.id ? "border-primary bg-primary/5" : "hover:bg-muted"
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-medium">{c.name}</span>
                <div className="flex gap-1">
                  <span className="text-xs text-muted-foreground">{c.role}</span>
                  <button
                    onClick={(e) => { e.stopPropagation(); handleDelete(c.id); }}
                    className="opacity-0 group-hover:opacity-100 hover:text-destructive"
                  >
                    <Trash2Icon className="size-3" />
                  </button>
                </div>
              </div>
              {c.description && (
                <p className="text-xs text-muted-foreground mt-1 line-clamp-2">{c.description}</p>
              )}
              {c.traits.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-2">
                  {c.traits.slice(0, 3).map((trait) => (
                    <span key={trait} className="text-[10px] bg-muted px-1.5 py-0.5 rounded">
                      {trait}
                    </span>
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
            <DialogTitle>{isEditing ? t.characters.edit : t.characters.create}</DialogTitle>
          </DialogHeader>
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
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>{t.common.cancel}</Button>
            <Button onClick={handleSave} disabled={!formName.trim()}>
              {isEditing ? t.characters.update : t.characters.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
