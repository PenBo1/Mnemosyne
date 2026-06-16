import { useState, useEffect, useCallback } from "react";
import { useWorkspaceStore } from "@/stores/workspace";
import { pickDirectory } from "@/services/workspaces";

export function useSidebarWorkspaces() {
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
    } finally {
      setCreating(false);
    }
  }, [newWorkspaceName, newWorkspacePath, addWorkspace]);

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
