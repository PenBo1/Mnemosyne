import { useState } from "react";
import { useI18n } from "@/locales/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Field, FieldLabel, FieldDescription, FieldError } from "@/components/ui/field";
import { PlusIcon, BookOpenIcon, LoaderIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import type { CreateBookRequest, PipelineBook } from "@/features/pipeline/types";

const PLATFORMS = ["tomato", "feilu", "qidian", "other"] as const;
const LANGUAGES = ["zh", "en"] as const;

interface BookEditorProps {
  existingBook?: PipelineBook;
  onSubmit: (req: CreateBookRequest) => Promise<void>;
  onCancel?: () => void;
  loading?: boolean;
  mode?: "create" | "edit";
}

export function BookEditor({
  existingBook,
  onSubmit,
  onCancel,
  loading = false,
  mode = "create",
}: BookEditorProps) {
  const { t } = useI18n();
  const [title, setTitle] = useState(existingBook?.title ?? "");
  const [platform, setPlatform] = useState<string>(existingBook?.platform ?? "tomato");
  const [genre, setGenre] = useState(existingBook?.genre ?? "");
  const [targetChapters, setTargetChapters] = useState<string>(
    existingBook?.target_chapters?.toString() ?? "100",
  );
  const [chapterWordCount, setChapterWordCount] = useState<string>(
    existingBook?.chapter_word_count?.toString() ?? "3000",
  );
  const [language, setLanguage] = useState<string>(existingBook?.language ?? "zh");
  const [authorIntent, setAuthorIntent] = useState("");
  const [externalContext, setExternalContext] = useState("");
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};
    if (!title.trim()) {
      newErrors.title = t.pipeline.errors.titleRequired;
    }
    if (!genre.trim()) {
      newErrors.genre = t.pipeline.errors.genreRequired;
    }
    const tc = parseInt(targetChapters, 10);
    if (isNaN(tc) || tc < 1 || tc > 9999) {
      newErrors.targetChapters = t.pipeline.errors.invalidChapterCount;
    }
    const cwc = parseInt(chapterWordCount, 10);
    if (isNaN(cwc) || cwc < 500 || cwc > 50000) {
      newErrors.chapterWordCount = t.pipeline.errors.invalidWordCount;
    }
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async () => {
    if (!validate()) return;
    const req: CreateBookRequest = {
      title: title.trim(),
      platform,
      genre: genre.trim(),
      target_chapters: parseInt(targetChapters, 10),
      chapter_word_count: parseInt(chapterWordCount, 10),
      language: language as "zh" | "en",
      author_intent: authorIntent.trim() || undefined,
      external_context: externalContext.trim() || undefined,
    };
    await onSubmit(req);
  };

  return (
    <Card className="p-4">
      <div className="flex items-center gap-2 mb-4">
        <BookOpenIcon className="size-5 text-muted-foreground" />
        <span className="font-medium">
          {mode === "create" ? t.pipeline.createBook : t.pipeline.editBook}
        </span>
      </div>

      <div className="flex flex-col gap-4">
        <Field>
          <FieldLabel>{t.pipeline.title}</FieldLabel>
          <Input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t.pipeline.titlePlaceholder}
            className={cn(errors.title && "border-destructive")}
          />
          {errors.title && <FieldError>{errors.title}</FieldError>}
        </Field>

        <Field>
          <FieldLabel>{t.pipeline.platform}</FieldLabel>
          <Select value={platform} onValueChange={setPlatform}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PLATFORMS.map((p) => (
                <SelectItem key={p} value={p}>
                  {t.pipeline.platforms[p]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>

        <Field>
          <FieldLabel>{t.pipeline.genre}</FieldLabel>
          <Input
            value={genre}
            onChange={(e) => setGenre(e.target.value)}
            placeholder={t.pipeline.genrePlaceholder}
            className={cn(errors.genre && "border-destructive")}
          />
          {errors.genre && <FieldError>{errors.genre}</FieldError>}
        </Field>

        <Field>
          <FieldLabel>{t.pipeline.targetChapters}</FieldLabel>
          <Input
            type="number"
            value={targetChapters}
            onChange={(e) => setTargetChapters(e.target.value)}
            min={1}
            max={9999}
            className={cn(errors.targetChapters && "border-destructive")}
          />
          <FieldDescription>{t.pipeline.targetChaptersDesc}</FieldDescription>
          {errors.targetChapters && <FieldError>{errors.targetChapters}</FieldError>}
        </Field>

        <Field>
          <FieldLabel>{t.pipeline.chapterWordCount}</FieldLabel>
          <Input
            type="number"
            value={chapterWordCount}
            onChange={(e) => setChapterWordCount(e.target.value)}
            min={500}
            max={50000}
            className={cn(errors.chapterWordCount && "border-destructive")}
          />
          <FieldDescription>{t.pipeline.chapterWordCountDesc}</FieldDescription>
          {errors.chapterWordCount && <FieldError>{errors.chapterWordCount}</FieldError>}
        </Field>

        <Field>
          <FieldLabel>{t.pipeline.language}</FieldLabel>
          <Select value={language} onValueChange={setLanguage}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {LANGUAGES.map((l) => (
                <SelectItem key={l} value={l}>
                  {t.pipeline.languages[l]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>

        <Field>
          <FieldLabel>{t.pipeline.authorIntent}</FieldLabel>
          <Textarea
            value={authorIntent}
            onChange={(e) => setAuthorIntent(e.target.value)}
            placeholder={t.pipeline.authorIntentPlaceholder}
            rows={3}
          />
          <FieldDescription>{t.pipeline.authorIntentDesc}</FieldDescription>
        </Field>

        <Field>
          <FieldLabel>{t.pipeline.externalContext}</FieldLabel>
          <Textarea
            value={externalContext}
            onChange={(e) => setExternalContext(e.target.value)}
            placeholder={t.pipeline.externalContextPlaceholder}
            rows={3}
          />
          <FieldDescription>{t.pipeline.externalContextDesc}</FieldDescription>
        </Field>

        <div className="flex items-center justify-end gap-2 mt-4">
          {onCancel && (
            <Button variant="outline" onClick={onCancel}>
              {t.common.cancel}
            </Button>
          )}
          <Button onClick={handleSubmit} disabled={loading}>
            {loading ? (
              <LoaderIcon className="size-4 animate-spin" />
            ) : (
              <PlusIcon className="size-4" />
            )}
            {mode === "create" ? t.pipeline.create : t.pipeline.save}
          </Button>
        </div>
      </div>
    </Card>
  );
}