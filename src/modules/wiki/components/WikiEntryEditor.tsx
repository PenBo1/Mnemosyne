import { useState, useEffect, useCallback } from "react";
import { cn } from "@/shared/utils";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { X } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import type { WikiEntry, WikiCategory, CreateWikiEntryRequest, UpdateWikiEntryRequest } from "@/shared/types";

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

  useEffect(() => {
    if (entry) {
      setTitle(entry.title);
      setContent(entry.content);
      setCategory(entry.category);
      setTags(entry.tags);
      setImportance(entry.importance);
      setSourceChapter(entry.source_chapter || undefined);
    } else if (isNew) {
      setTitle("");
      setContent("");
      setCategory("general");
      setTags([]);
      setImportance(0);
      setSourceChapter(undefined);
    }
  }, [entry, isNew]);

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
      <CardContent className="flex-1 flex flex-col gap-4 pt-4">
        {/* 标题 */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium">{t.knowledge.title_label}</label>
          <Input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t.knowledge.titlePlaceholder}
            className="h-8"
          />
        </div>

        {/* 分类 */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium">{t.knowledge.category}</label>
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
        </div>

        {/* 内容 */}
        <div className="flex flex-col gap-1.5 flex-1 min-h-0">
          <label className="text-xs font-medium">{t.knowledge.content}</label>
          <Textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder={t.knowledge.contentPlaceholder}
            className="flex-1 min-h-[200px] resize-none"
          />
        </div>

        {/* 标签 */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium">{t.knowledge.tags}</label>
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
        </div>

        {/* 重要性 */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium">{t.wiki.importance}</label>
          <div className="flex gap-1">
            {[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map((level) => (
              <Button
                key={level}
                variant={importance === level ? "default" : "outline"}
                size="sm"
                onClick={() => setImportance(level)}
                className="h-7 w-7 text-xs p-0"
              >
                {level}
              </Button>
            ))}
          </div>
        </div>

        {/* 来源章节 */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium">{t.wiki.sourceChapter}</label>
          <Input
            type="number"
            value={sourceChapter || ""}
            onChange={(e) => setSourceChapter(e.target.value ? parseInt(e.target.value) : undefined)}
            placeholder={t.wiki.optional}
            className="h-8 w-20"
            min={1}
          />
        </div>

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
      </CardContent>
    </Card>
  );
}
