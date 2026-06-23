import { useState, useEffect, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useI18n } from "@/lib/i18n";
import type { WikiEntry, WikiCategory, CreateWikiEntryRequest, UpdateWikiEntryRequest } from "@/types";

const WIKI_CATEGORIES: WikiCategory[] = ["character", "location", "event", "concept", "item", "other"];

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
  const [category, setCategory] = useState<WikiCategory>(entry?.category || "other");
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
      setCategory("other");
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
        {/* Title */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium">{t.knowledge.title_label}</label>
          <Input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t.knowledge.titlePlaceholder}
            className="h-8"
          />
        </div>

        {/* Category */}
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

        {/* Content */}
        <div className="flex flex-col gap-1.5 flex-1 min-h-0">
          <label className="text-xs font-medium">{t.knowledge.content}</label>
          <Textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder={t.knowledge.contentPlaceholder}
            className="flex-1 min-h-[200px] resize-none"
          />
        </div>

        {/* Tags */}
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
              Add
            </Button>
          </div>
          {tags.length > 0 && (
            <div className="flex gap-1 flex-wrap mt-1">
              {tags.map((tag) => (
                <span
                  key={tag}
                  className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-muted text-xs cursor-pointer hover:bg-muted/80"
                  onClick={() => handleRemoveTag(tag)}
                >
                  {tag}
                  <span className="text-muted-foreground">×</span>
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Importance */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium">Importance (0-10)</label>
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

        {/* Source Chapter */}
        <div className="flex flex-col gap-1.5">
          <label className="text-xs font-medium">Source Chapter</label>
          <Input
            type="number"
            value={sourceChapter || ""}
            onChange={(e) => setSourceChapter(e.target.value ? parseInt(e.target.value) : undefined)}
            placeholder="Optional"
            className="h-8 w-20"
            min={1}
          />
        </div>

        {/* Actions */}
        <div className="flex gap-2 justify-end pt-2 border-t">
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