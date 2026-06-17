import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import { useWorkspaceStore } from "@/stores/workspace";
import { pickDirectory } from "@/services/workspaces";

export function useSidebarWorkspaces() {
  const { t } = useI18n();
  const { workspaces, loadWorkspaces, addWorkspace, removeWorkspace, setActiveWorkspace, activeWorkspaceId } =
    useWorkspaceStore();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [newWorkspaceName, setNewWorkspaceName] = useState("");
  const [newWorkspacePath, setNewWorkspacePath] = useState("");
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    loadWorkspaces();
  }, [loadWorkspaces]);

  const handlePickDirectory = useCallback(async () => {
    const selected = await pickDirectory();
    if (selected) {
      setNewWorkspacePath(selected);
      if (!newWorkspaceName) {
        const folderName = selected.split(/[\\/]/).pop() || "";
        setNewWorkspaceName(folderName);
      }
    }
  }, [newWorkspaceName]);

  const handleAddWorkspace = useCallback(async () => {
    if (!newWorkspaceName.trim() || !newWorkspacePath) return;
    setCreating(true);
    try {
      await addWorkspace(newWorkspaceName.trim(), newWorkspacePath);
      setDialogOpen(false);
      setNewWorkspaceName("");
      setNewWorkspacePath("");
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    } finally {
      setCreating(false);
    }
  }, [newWorkspaceName, newWorkspacePath, addWorkspace, t.common.createdSuccessfully, t.common.failedToCreate]);

  return {
    workspaces,
    activeWorkspaceId,
    setActiveWorkspace,
    removeWorkspace,
    dialogOpen,
    setDialogOpen,
    newWorkspaceName,
    setNewWorkspaceName,
    newWorkspacePath,
    setNewWorkspacePath,
    creating,
    handlePickDirectory,
    handleAddWorkspace,
  };
}
