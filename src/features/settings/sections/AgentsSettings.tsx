import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { PlusIcon, PencilIcon, Trash2Icon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { useCustomInstructions, useSnippets } from "@/features/settings/hooks";
import { SettingsSection } from "@/features/settings/components/settings-section";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { isValidHandle, normalizeHandle, type Snippet } from "@/services/settings";
import { AgentIdentityBlock } from "@/features/settings/sections/AgentIdentityBlock";

export function AgentsSettings() {
  const { t } = useI18n();
  const customInstructions = useCustomInstructions();
  const snippets = useSnippets();
  const [editing, setEditing] = useState<Snippet | null>(null);

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.agents}</PageTitle>
          <PageDescription>{t.settings.agentsDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      <CustomInstructionsBlock
        key={customInstructions.loaded ? 'loaded' : 'loading'}
        value={customInstructions.value}
        loaded={customInstructions.loaded}
        onSave={(text) => void customInstructions.save(text)}
      />

      <AgentIdentityBlock />

      <SettingsSection title={t.agents.snippets} description={t.agents.snippetsDesc}>
        <div className="flex flex-col gap-3 px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex flex-col gap-0.5">
              <span className="text-sm font-medium">{t.agents.snippets}</span>
              <span className="text-xs text-muted-foreground">{t.agents.snippetsDesc}</span>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() =>
                setEditing({
                  id: snippets.newId(),
                  handle: "",
                  name: "",
                  description: "",
                  content: "",
                })
              }
            >
              <PlusIcon data-icon="inline-start" />
              {t.agents.newSnippet}
            </Button>
          </div>

          {snippets.snippets.length === 0 ? (
            <p className="rounded-lg border border-dashed px-4 py-6 text-center text-xs text-muted-foreground">
              {t.agents.snippetEmpty}
            </p>
          ) : (
            <ul className="flex flex-col gap-1.5">
              {snippets.snippets.map((s) => (
                <li
                  key={s.id}
                  className="flex items-center gap-2 rounded-lg border px-3 py-2"
                >
                  <code className="rounded bg-[var(--bg-overlay-l1)] px-1.5 py-0.5 font-mono text-[11px] text-muted-foreground">
                    #{s.handle}
                  </code>
                  <div className="flex min-w-0 flex-1 flex-col">
                    <span className="truncate text-xs font-medium">{s.name}</span>
                    {s.description && (
                      <span className="truncate text-[11px] text-muted-foreground">
                        {s.description}
                      </span>
                    )}
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => setEditing(s)}
                    title={t.agents.editSnippet}
                  >
                    <PencilIcon className="size-3.5" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-destructive hover:text-destructive"
                    onClick={() => void snippets.remove(s.id)}
                    title={t.common.delete}
                  >
                    <Trash2Icon className="size-3.5" />
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </SettingsSection>

      <SnippetEditorDialog
        key={editing?.id ?? 'none'}
        snippet={editing}
        existing={snippets.snippets}
        onClose={() => setEditing(null)}
        onSave={async (s) => {
          await snippets.upsert(s);
          setEditing(null);
        }}
      />
    </PageContainer>
  );
}

function CustomInstructionsBlock({
  value,
  loaded,
  onSave,
}: {
  value: string;
  loaded: boolean;
  onSave: (text: string) => void;
}) {
  const { t } = useI18n();
  // 用 key 重置后：每次 loaded 状态变化时父组件重挂，初始值即 value
  const [draft, setDraft] = useState(value);

  return (
    <SettingsSection title={t.agents.customInstructions} description={t.agents.customInstructionsDesc}>
      <div className="flex flex-col gap-2 px-4 py-3">
        <div className="flex items-center justify-between">
          <div className="flex flex-col gap-0.5">
            <span className="text-sm font-medium">{t.agents.customInstructions}</span>
            <span className="text-xs text-muted-foreground">{t.agents.customInstructionsDesc}</span>
          </div>
          {loaded && draft !== value && (
            <Button size="sm" onClick={() => onSave(draft)}>
              {t.common.save}
            </Button>
          )}
        </div>
        <Textarea
          value={draft}
          onChange={(e) => {
            setDraft(e.target.value);
          }}
          placeholder={t.agents.customInstructionsPlaceholder}
          className="min-h-[100px] resize-y text-sm"
        />
      </div>
    </SettingsSection>
  );
}

function SnippetEditorDialog({
  snippet,
  existing,
  onClose,
  onSave,
}: {
  snippet: Snippet | null;
  existing: Snippet[];
  onClose: () => void;
  onSave: (s: Snippet) => void;
}) {
  const { t } = useI18n();
  // 用 key 重置后：snippet 变化时父组件重挂，初始值即 snippet
  const [draft, setDraft] = useState<Snippet | null>(snippet);

  if (!draft) return null;

  const isNew = !existing.some((s) => s.id === draft.id);
  const handleErr = !draft.handle
    ? t.agents.snippetHandleErrorRequired
    : !isValidHandle(draft.handle)
      ? t.agents.snippetHandleErrorInvalid
      : existing.some((s) => s.id !== draft.id && s.handle === draft.handle)
        ? t.agents.snippetHandleErrorUsed
        : null;
  const canSave = !handleErr && draft.name.trim().length > 0 && draft.content.trim().length > 0;

  return (
    <Dialog open={!!snippet} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{isNew ? t.agents.newSnippet : t.agents.editSnippet}</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-3">
          <div className="flex gap-2">
            <div className="flex w-32 flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">
                {t.agents.snippetHandle}
              </span>
              <div className="relative">
                <span className="absolute top-1/2 left-2 -translate-y-1/2 font-mono text-xs text-muted-foreground">
                  #
                </span>
                <Input
                  value={draft.handle}
                  onChange={(e) =>
                    setDraft({ ...draft, handle: normalizeHandle(e.target.value) })
                  }
                  placeholder={t.agents.snippetHandlePlaceholder}
                  className="h-8 pl-5 font-mono text-xs"
                />
              </div>
              {handleErr && (
                <span className="text-[10px] text-destructive">{handleErr}</span>
              )}
            </div>
            <div className="flex flex-1 flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">
                {t.agents.snippetName}
              </span>
              <Input
                value={draft.name}
                onChange={(e) => setDraft({ ...draft, name: e.target.value })}
                placeholder={t.agents.snippetNamePlaceholder}
                className="h-8 text-xs"
              />
            </div>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-xs font-medium text-muted-foreground">
              {t.agents.snippetDescription}
            </span>
            <Input
              value={draft.description}
              onChange={(e) => setDraft({ ...draft, description: e.target.value })}
              placeholder={t.agents.snippetDescriptionPlaceholder}
              className="h-8 text-xs"
            />
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-xs font-medium text-muted-foreground">
              {t.agents.snippetContent}
            </span>
            <Textarea
              value={draft.content}
              onChange={(e) => setDraft({ ...draft, content: e.target.value })}
              placeholder={t.agents.snippetContentPlaceholder}
              className="min-h-40 resize-y font-mono text-xs"
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t.common.cancel}
          </Button>
          <Button disabled={!canSave} onClick={() => onSave(draft)}>
            {t.common.save}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}