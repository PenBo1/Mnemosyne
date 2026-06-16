import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { PlusIcon, FolderOpenIcon } from "lucide-react";
import { useI18n } from "@/lib/i18n";

interface CreateWorkspaceDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  name: string;
  onNameChange: (name: string) => void;
  path: string;
  onPathChange: (path: string) => void;
  creating: boolean;
  onPickDirectory: () => void;
  onCreate: () => void;
}

export function CreateWorkspaceDialog({
  open,
  onOpenChange,
  name,
  onNameChange,
  path,
  onPathChange,
  creating,
  onPickDirectory,
  onCreate,
}: CreateWorkspaceDialogProps) {
  const { t } = useI18n();

  return (
    <Dialog open={open} onOpenChange={(o) => {
      onOpenChange(o);
      if (!o) {
        onNameChange("");
        onPathChange("");
      }
    }}>
      <DialogTrigger asChild>
        <button className="rounded-md p-0.5 hover:bg-sidebar-accent">
          <PlusIcon className="size-3.5" />
        </button>
      </DialogTrigger>
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
              onChange={(e) => onNameChange(e.target.value)}
              placeholder={t.sidebar.workspaceNamePlaceholder}
              onKeyDown={(e) => {
                if (e.key === "Enter" && path) onCreate();
              }}
            />
          </Field>
          <Field>
            <FieldLabel>{t.sidebar.workspace}</FieldLabel>
            <div className="flex gap-2">
              <Input
                value={path}
                onChange={(e) => onPathChange(e.target.value)}
                placeholder={t.sidebar.selectDirectory}
                readOnly
              />
              <Button variant="outline" onClick={onPickDirectory} type="button">
                <FolderOpenIcon />
              </Button>
            </div>
          </Field>
        </FieldGroup>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t.common.cancel}
          </Button>
          <Button onClick={onCreate} disabled={!name.trim() || !path || creating}>
            {t.common.create}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
