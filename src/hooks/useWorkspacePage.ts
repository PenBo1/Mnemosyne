import { useState, useEffect, useCallback } from "react";
import { useWorkspaceStore } from "@/stores/workspace";
import { pickDirectory } from "@/services/workspaces";

export function useWorkspacePage() {
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
    } finally {
      setLoading(false);
    }
  }, [name, path, addWorkspace]);

  const handleDelete = useCallback(async (id: string) => {
    await removeWorkspace(id);
  }, [removeWorkspace]);

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
