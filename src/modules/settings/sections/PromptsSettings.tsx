import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { PlusIcon, MoreVerticalIcon, PencilIcon, Trash2Icon, MessageSquareIcon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { usePrompts } from "@/features/settings/hooks/usePrompts";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState, EmptyState } from "@/components/shared/state";

const CATEGORIES = ["general", "writing", "character", "world", "dialogue", "style"];

export function PromptsSettings() {
  const [filterCategory, setFilterCategory] = useState<string>("all");
  const { prompts, loading, create, update, remove } = usePrompts(filterCategory === "all" ? undefined : filterCategory);
  const { t } = useI18n();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingPrompt, setEditingPrompt] = useState<{ id: string; name: string; content: string; category: string } | null>(null);
  const [name, setName] = useState("");
  const [content, setContent] = useState("");
  const [category, setCategory] = useState("general");

  async function handleSave() {
    if (editingPrompt) {
      await update(editingPrompt.id, name, content, category);
    } else {
      await create(name, content, category);
    }
    setDialogOpen(false);
    setEditingPrompt(null);
    setName("");
    setContent("");
    setCategory("general");
  }

  async function handleDelete(id: string) {
    await remove(id);
  }

  function openEdit(prompt: { id: string; name: string; content: string; category: string }) {
    setEditingPrompt(prompt);
    setName(prompt.name);
    setContent(prompt.content);
    setCategory(prompt.category);
    setDialogOpen(true);
  }

  function openCreate() {
    setEditingPrompt(null);
    setName("");
    setContent("");
    setCategory("general");
    setDialogOpen(true);
  }

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.prompts}</PageTitle>
          <PageDescription>{t.settings.promptsDesc}</PageDescription>
        </PageHeading>
        <PageActions>
          <Select value={filterCategory} onValueChange={setFilterCategory}>
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t.prompts.allCategories}</SelectItem>
              {CATEGORIES.map((cat) => (
                <SelectItem key={cat} value={cat}>
                  {t.prompts.categories[cat as keyof typeof t.prompts.categories]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
            <DialogTrigger asChild>
              <Button onClick={openCreate}>
                <PlusIcon data-icon="inline-start" />
                <span>{t.prompts.create}</span>
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>{editingPrompt ? t.prompts.edit : t.prompts.create}</DialogTitle>
                <DialogDescription>{t.prompts.contentPlaceholder}</DialogDescription>
              </DialogHeader>
              <FieldGroup>
                <Field>
                  <FieldLabel>{t.prompts.name}</FieldLabel>
                  <Input value={name} onChange={(e) => setName(e.target.value)} placeholder={t.prompts.namePlaceholder} />
                </Field>
                <Field>
                  <FieldLabel>{t.prompts.category}</FieldLabel>
                  <Select value={category} onValueChange={setCategory}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {CATEGORIES.map((cat) => (
                        <SelectItem key={cat} value={cat}>
                          {t.prompts.categories[cat as keyof typeof t.prompts.categories]}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
                <Field>
                  <FieldLabel>{t.prompts.content}</FieldLabel>
                  <Textarea
                    value={content}
                    onChange={(e) => setContent(e.target.value)}
                    placeholder={t.prompts.contentPlaceholder}
                    rows={8}
                  />
                </Field>
              </FieldGroup>
              <DialogFooter>
                <Button variant="outline" onClick={() => setDialogOpen(false)}>
                  {t.prompts.cancel}
                </Button>
                <Button onClick={handleSave} disabled={!name || !content}>
                  {editingPrompt ? t.prompts.update : t.prompts.save}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </PageActions>
      </PageHeader>

      {loading ? (
        <LoadingState label={t.common.loading} />
      ) : prompts.length === 0 ? (
        <EmptyState
          icon={<MessageSquareIcon className="size-6" />}
          title={t.prompts.empty}
          description={t.prompts.create}
        >
          <Button onClick={openCreate}>
            <PlusIcon data-icon="inline-start" />
            {t.prompts.create}
          </Button>
        </EmptyState>
      ) : (
        <Card className="py-0 gap-0">
          <CardContent className="divide-y px-0">
            {prompts.map((prompt) => (
              <div key={prompt.id} className="flex flex-col gap-2 px-4 py-3 transition-colors hover:bg-muted/50">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">{prompt.name}</span>
                    <Badge variant="secondary" className="text-xs">{prompt.category}</Badge>
                  </div>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon-sm">
                        <MoreVerticalIcon />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={() => openEdit(prompt)}>
                        <PencilIcon />
                        <span>{t.common.edit}</span>
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => handleDelete(prompt.id)} className="text-destructive">
                        <Trash2Icon />
                        <span>{t.common.delete}</span>
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
                <p className="line-clamp-3 whitespace-pre-wrap text-xs text-muted-foreground">
                  {prompt.content}
                </p>
              </div>
            ))}
          </CardContent>
        </Card>
      )}
    </PageContainer>
  );
}
