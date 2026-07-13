import { useState, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Slider } from "@/components/ui/slider";
import { X } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type { WikiEntry, WikiCategory, CreateWikiEntryRequest, UpdateWikiEntryRequest } from "@/features/wiki/types";

const WIKI_CATEGORIES: WikiCategory[] = ["general", "character", "location", "event", "concept", "reference"];

interface WikiEntryEditorProps {
  entry?: WikiEntry | null;
  onSave: (request: CreateWikiEntryRequest | UpdateWikiEntryRequest) => void;
  onCancel: () => void;
  isNew?: boolean;
  className?: string;
}

export function WikiEntryEditor({ entry, onSave, onCancel, isNew = false, className }: WikiEntryEditorProps) {
  const { t } = useI18n();
  const [title, setTitle] = useState(entry?.title || "");
  const [content, setContent] = useState(entry?.content || "");
  const [category, setCategory] = useState<WikiCategory>(entry?.category || "general");
  const [tags, setTags] = useState<string[]>(entry?.tags || []);
  const [importance, setImportance] = useState(entry?.importance || 0);
  const [sourceChapter, setSourceChapter] = useState<number | undefined>(entry?.source_chapter || undefined);
  const [tagInput, setTagInput] = useState("");

  const handleAddTag = useCallback(() => {
    const trimmed = tagInput.trim();
    if (trimmed && !tags.includes(trimmed)) {
      setTags([...tags, trimmed]);
      setTagInput("");
    }
  }, [tagInput, tags]);

  const handleRemoveTag = useCallback((tagToRemove: string) => {
    setTags(tags.filter((tag) => tag !== tagToRemove));
  }, [tags]);

  const handleSave = useCallback(() => {
    if (!title.trim()) return;

    const request: CreateWikiEntryRequest | UpdateWikiEntryRequest = {
      title: title.trim(),
      content: content.trim(),
      category,
      tags,
      importance,
      source_chapter: sourceChapter,
    };

    onSave(request);
  }, [title, content, category, tags, importance, sourceChapter, onSave]);

  return (
    <Card className={cn("flex flex-col", className)}>
      <CardHeader className="border-b">
        <CardTitle>
          {isNew ? t.knowledge.create : t.knowledge.edit}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 flex flex-col pt-4">
        <FieldGroup className="flex-1">
          {/* 标题 */}
          <Field>
            <FieldLabel>{t.knowledge.title_label}</FieldLabel>
            <Input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder={t.knowledge.titlePlaceholder}
              className="h-8"
            />
          </Field>

          {/* 分类 */}
          <Field>
            <FieldLabel>{t.knowledge.category}</FieldLabel>
            <div className="flex gap-1 flex-wrap">
              {WIKI_CATEGORIES.map((cat) => (
                <Button
                  key={cat}
                  variant={category === cat ? "default" : "outline"}
                  size="sm"
                  onClick={() => setCategory(cat)}
                  className="h-7 px-2 text-xs"
                >
                  {cat}
                </Button>
              ))}
            </div>
          </Field>

          {/* 内容 */}
          <Field className="flex-1 min-h-0">
            <FieldLabel>{t.knowledge.content}</FieldLabel>
            <Textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              placeholder={t.knowledge.contentPlaceholder}
              className="flex-1 min-h-[200px] resize-none"
            />
          </Field>

          {/* 标签 */}
          <Field>
            <FieldLabel>{t.knowledge.tags}</FieldLabel>
            <div className="flex gap-2 items-center">
              <Input
                value={tagInput}
                onChange={(e) => setTagInput(e.target.value)}
                placeholder={t.knowledge.tagsPlaceholder}
                className="h-8 flex-1"
                onKeyDown={(e: React.KeyboardEvent<HTMLInputElement>) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    handleAddTag();
                  }
                }}
              />
              <Button variant="outline" size="sm" onClick={handleAddTag} className="h-8">
                {t.common.add}
              </Button>
            </div>
            {tags.length > 0 && (
              <div className="flex gap-1 flex-wrap">
                {tags.map((tag) => (
                  <Badge
                    key={tag}
                    variant="secondary"
                    className="cursor-pointer gap-1"
                    onClick={() => handleRemoveTag(tag)}
                  >
                    {tag}
                    <X className="size-3" />
                  </Badge>
                ))}
              </div>
            )}
          </Field>

          {/* 重要性 */}
          <Field>
            <FieldLabel>{t.wiki.importance}</FieldLabel>
            <div className="flex items-center gap-2">
              <Slider
                min={0}
                max={10}
                step={1}
                value={[importance]}
                onValueChange={([v]) => setImportance(v)}
              />
              <span className="text-xs text-muted-foreground w-6 text-right">{importance}</span>
            </div>
          </Field>

          {/* 来源章节 */}
          <Field>
            <FieldLabel>{t.wiki.sourceChapter}</FieldLabel>
            <Input
              type="number"
              value={sourceChapter || ""}
              onChange={(e) => setSourceChapter(e.target.value ? parseInt(e.target.value) : undefined)}
              placeholder={t.wiki.optional}
              className="h-8 w-20"
              min={1}
            />
          </Field>

          {/* 操作 */}
          <Separator />
          <div className="flex gap-2 justify-end pt-2">
            <Button variant="outline" size="sm" onClick={onCancel}>
              {t.common.cancel}
            </Button>
            <Button size="sm" onClick={handleSave} disabled={!title.trim()}>
              {isNew ? t.knowledge.save : t.knowledge.update}
            </Button>
          </div>
        </FieldGroup>
      </CardContent>
    </Card>
  );
}
