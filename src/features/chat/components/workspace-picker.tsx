import { useState } from "react";
import { PlusIcon, FolderOpenIcon, FolderIcon } from "lucide-react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { pickDirectory } from "@/features/workspace/services";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

/** 新建工作区专属标识 —— 与真实 workspace id 区分（UUID 不会以 _ 开头） */
const NEW_WORKSPACE_VALUE = "__new_workspace__";

/**
 * 工作区选择器：shadcn Select 下拉 + 内置「新建工作区」弹窗。
 *
 * - 自包含：直接读写 useWorkspaceStore，不依赖父组件传参
 * - 新建工作区复用 pickDirectory + addWorkspace，与侧边栏逻辑一致
 * - 选择 __new_workspace__ 时不切换激活工作区，而是打开弹窗
 */
export function WorkspacePicker() {
  const { t } = useI18n();
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const setActiveWorkspace = useWorkspaceStore((s) => s.setActiveWorkspace);
  const addWorkspace = useWorkspaceStore((s) => s.addWorkspace);

  const [dialogOpen, setDialogOpen] = useState(false);
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [creating, setCreating] = useState(false);

  const handleValueChange = (value: string) => {
    if (value === NEW_WORKSPACE_VALUE) {
      setDialogOpen(true);
      return;
    }
    setActiveWorkspace(value);
  };

  const handlePickDirectory = async () => {
    const selected = await pickDirectory();
    if (selected) {
      setPath(selected);
      if (!name) {
        const folderName = selected.split(/[\\/]/).pop() || "";
        setName(folderName);
      }
    }
  };

  const handleCreate = async () => {
    if (!name.trim() || !path) return;
    setCreating(true);
    try {
      await addWorkspace(name.trim(), path);
      setDialogOpen(false);
      setName("");
      setPath("");
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    } finally {
      setCreating(false);
    }
  };

  const handleOpenChange = (open: boolean) => {
    setDialogOpen(open);
    if (!open) {
      setName("");
      setPath("");
    }
  };

  return (
    <>
      <Select value={activeWorkspaceId ?? ""} onValueChange={handleValueChange}>
        <SelectTrigger size="sm" className="min-w-36 max-w-48">
          <FolderIcon className="text-muted-foreground" />
          <SelectValue placeholder={t.agentChat.selectWorkspace} />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectLabel>{t.sidebar.workspaces}</SelectLabel>
            {workspaces.map((ws) => (
              <SelectItem key={ws.id} value={ws.id}>
                {ws.name}
              </SelectItem>
            ))}
            {workspaces.length === 0 && (
              <div className="px-2 py-1.5 text-xs text-muted-foreground">
                {t.agentChat.noWorkspace}
              </div>
            )}
          </SelectGroup>
          <SelectSeparator />
          <SelectItem value={NEW_WORKSPACE_VALUE}>
            <PlusIcon />
            {t.sidebar.newWorkspace}
          </SelectItem>
        </SelectContent>
      </Select>

      {/* 新建工作区弹窗（受控，无 trigger） */}
      <Dialog open={dialogOpen} onOpenChange={handleOpenChange}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t.sidebar.createWorkspace}</DialogTitle>
            <DialogDescription>{t.sidebar.createWorkspaceDesc}</DialogDescription>
          </DialogHeader>
          <FieldGroup>
            <Field>
              <FieldLabel>{t.sidebar.workspaceNamePlaceholder}</FieldLabel>
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={t.sidebar.workspaceNamePlaceholder}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && path) void handleCreate();
                }}
              />
            </Field>
            <Field>
              <FieldLabel>{t.sidebar.workspace}</FieldLabel>
              <div className="flex gap-2">
                <Input
                  value={path}
                  onChange={(e) => setPath(e.target.value)}
                  placeholder={t.sidebar.selectDirectory}
                  readOnly
                />
                <Button variant="outline" onClick={() => void handlePickDirectory()} type="button">
                  <FolderOpenIcon className="size-4" />
                </Button>
              </div>
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              {t.common.cancel}
            </Button>
            <Button onClick={() => void handleCreate()} disabled={!name.trim() || !path || creating}>
              {t.common.create}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
