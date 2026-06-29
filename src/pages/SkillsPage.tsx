import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { Spinner } from "@/components/ui/spinner";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
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
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState } from "@/components/shared/state";
import { PuzzleIcon, RefreshCwIcon, WrenchIcon, PlusIcon, PencilIcon, Trash2Icon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { useSkills } from "@/features/skill/hooks/useSkills";
import { SKILL_CATEGORIES } from "@/shared/types";
import type { SkillMeta, Skill } from "@/shared/types";

export function SkillsPage() {
  const { t } = useI18n();
  const { skills, loading, filterCategory, setFilterCategory, refresh, getSkill, create, update, remove } = useSkills();

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingSkill, setEditingSkill] = useState<Skill | null>(null);
  const [skillName, setSkillName] = useState("");
  const [skillDescription, setSkillDescription] = useState("");
  const [skillCategory, setSkillCategory] = useState("general");
  const [skillContent, setSkillContent] = useState("");
  const [saving, setSaving] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);

  function openCreateDialog() {
    setEditingSkill(null);
    setSkillName("");
    setSkillDescription("");
    setSkillCategory("general");
    setSkillContent("");
    setDialogOpen(true);
  }

  async function openEditDialog(skill: SkillMeta) {
    try {
      const fullSkill = await getSkill(skill.name);
      setEditingSkill(fullSkill);
      setSkillName(fullSkill.name);
      setSkillDescription(fullSkill.description);
      setSkillCategory(fullSkill.category);
      setSkillContent(fullSkill.content);
      setDialogOpen(true);
    } catch {
      // 错误由 hook 处理
    }
  }

  async function handleSave() {
    if (!skillName.trim()) return;
    setSaving(true);
    try {
      const params = {
        name: skillName,
        description: skillDescription,
        category: skillCategory,
        content: skillContent,
      };
      if (editingSkill) {
        await update(params);
      } else {
        await create(params);
      }
      setDialogOpen(false);
    } catch {
      // 错误由 hook 处理
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(name: string) {
    try {
      await remove(name);
      setDeleteConfirm(null);
    } catch {
      // 错误由 hook 处理
    }
  }

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <PuzzleIcon />
            {t.skills.title}
          </PageTitle>
          <PageDescription>{t.skills.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Select value={filterCategory} onValueChange={setFilterCategory}>
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t.skills.allCategories}</SelectItem>
              {SKILL_CATEGORIES.map((cat) => (
                <SelectItem key={cat} value={cat}>
                  {t.skills.categories[cat as keyof typeof t.skills.categories]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button variant="outline" size="sm" onClick={refresh} disabled={loading}>
            <RefreshCwIcon className={`size-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
          <Button size="sm" onClick={openCreateDialog}>
            <PlusIcon data-icon="inline-start" />
            {t.skills.add}
          </Button>
        </PageActions>
      </PageHeader>

      {loading ? (
        <LoadingState label={t.common.loading} />
      ) : skills.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <PuzzleIcon />
            </EmptyMedia>
            <EmptyTitle>{t.skills.empty}</EmptyTitle>
            <EmptyDescription>{t.skills.description}</EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={openCreateDialog} size="sm">
              <PlusIcon className="size-4" />
            {t.skills.add}
            </Button>
          </EmptyContent>
        </Empty>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {skills.map((skill) => (
            <Card key={skill.name} className="group transition-shadow hover:shadow-md">
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <CardTitle className="truncate text-base">{skill.name}</CardTitle>
                    <CardDescription className="flex items-center gap-2">
                      <Badge variant="secondary">
                        {t.skills.categories[skill.category as keyof typeof t.skills.categories] || skill.category}
                      </Badge>
                    </CardDescription>
                  </div>
                  <div className="flex items-center gap-1">
                    <Button variant="ghost" size="icon-sm" onClick={() => openEditDialog(skill)}>
                      <PencilIcon className="size-3.5" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      onClick={() => setDeleteConfirm(skill.name)}
                      className="text-destructive hover:text-destructive opacity-0 group-hover:opacity-100 transition-opacity"
                    >
                      <Trash2Icon className="size-3.5" />
                    </Button>
                  </div>
                </div>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <p className="line-clamp-2 text-sm text-muted-foreground">{skill.description}</p>
                {skill.requires_tools.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    {skill.requires_tools.map((tool) => (
                      <Badge key={tool} variant="outline" className="text-[10px] gap-1">
                        <WrenchIcon className="size-2.5" />
                        {tool}
                      </Badge>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* 创建/编辑对话框 */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{editingSkill ? t.skills.edit : t.skills.add}</DialogTitle>
            <DialogDescription>{t.skills.description}</DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.skills.name}</FieldLabel>
              <Input
                value={skillName}
                onChange={(e) => setSkillName(e.target.value)}
                placeholder={t.skills.namePlaceholder}
                disabled={!!editingSkill}
              />
            </Field>
            <Field>
              <FieldLabel>{t.skills.description}</FieldLabel>
              <Input
                value={skillDescription}
                onChange={(e) => setSkillDescription(e.target.value)}
                placeholder={t.skills.descriptionPlaceholder}
              />
            </Field>
            <Field>
              <FieldLabel>{t.skills.category}</FieldLabel>
              <Select value={skillCategory} onValueChange={setSkillCategory}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {SKILL_CATEGORIES.map((cat) => (
                    <SelectItem key={cat} value={cat}>{cat}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field>
              <FieldLabel>{t.skills.content}</FieldLabel>
              <Textarea
                value={skillContent}
                onChange={(e) => setSkillContent(e.target.value)}
                placeholder="# Skill Instructions&#10;&#10;Write your skill instructions here..."
                className="min-h-[200px] font-mono"
              />
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              {t.skills.cancel}
            </Button>
            <Button onClick={handleSave} disabled={!skillName.trim() || saving}>
              {saving ? <Spinner className="size-4" /> : null}
              {editingSkill ? t.skills.update : t.skills.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 删除确认对话框 */}
      <Dialog open={!!deleteConfirm} onOpenChange={() => setDeleteConfirm(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t.skills.deleteConfirm}</DialogTitle>
            <DialogDescription>
              {t.skills.deleteConfirmDesc}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteConfirm(null)}>
              {t.skills.cancel}
            </Button>
            <Button variant="destructive" onClick={() => deleteConfirm && handleDelete(deleteConfirm)}>
              {t.skills.delete}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageContainer>
  );
}
