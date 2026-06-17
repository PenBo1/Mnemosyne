import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import { useWorkspaceStore } from "@/stores/workspace";
import { pickDirectory } from "@/services/workspaces";

export function useWorkspacePage() {
  const { t } = useI18n();
  const { workspaces, loadWorkspaces, addWorkspace, removeWorkspace } = useWorkspaceStore();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadWorkspaces();
  }, [loadWorkspaces]);

  const handlePickDirectory = useCallback(async () => {
    const selected = await pickDirectory();
    if (selected) {
      setPath(selected);
      if (!name) {
        const folderName = selected.split(/[\\/]/).pop() || "";
        setName(folderName);
      }
    }
  }, [name]);

  const handleCreate = useCallback(async () => {
    if (!name || !path) return;
    setLoading(true);
    try {
      await addWorkspace(name, path);
      setDialogOpen(false);
      setName("");
      setPath("");
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    } finally {
      setLoading(false);
    }
  }, [name, path, addWorkspace, t.common.createdSuccessfully, t.common.failedToCreate]);

  const handleDelete = useCallback(async (id: string) => {
    try {
      await removeWorkspace(id);
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
  }, [removeWorkspace, t.common.deletedSuccessfully, t.common.failedToDelete]);

  return {
    workspaces,
    dialogOpen,
    setDialogOpen,
    name,
    setName,
    path,
    loading,
    handlePickDirectory,
    handleCreate,
    handleDelete,
  };
}
