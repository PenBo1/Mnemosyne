import { useState, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/shared/i18n";
import { gitGetConfig, gitSetConfig, gitCheckInstalled } from "@/features/settings/services";
import type { GitConfig } from "@/shared/types";

const DEFAULT_CONFIG: GitConfig = {
  user_name: null,
  user_email: null,
  auto_stage: false,
  commit_message_template: null,
  enable_remote: false,
};

export function useGitSettings() {
  const { t } = useI18n();
  const [config, setConfig] = useState<GitConfig | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [gitInstalled, setGitInstalled] = useState<boolean | null>(null);

  const loadConfig = useCallback(async (workspacePath: string) => {
    setLoading(true);
    try {
      const data = await gitGetConfig(workspacePath);
      setConfig(data);
    } catch (err) {
      console.error("Failed to load git config:", err);
      setConfig(null);
    } finally {
      setLoading(false);
    }
  }, []);

  const saveConfig = useCallback(
    async (workspacePath: string, newConfig: GitConfig) => {
      setSaving(true);
      try {
        await gitSetConfig(workspacePath, newConfig);
        setConfig(newConfig);
        toast.success(t.common.updatedSuccessfully);
      } catch (err) {
        console.error("Failed to save git config:", err);
        toast.error(t.common.failedToUpdate);
      } finally {
        setSaving(false);
      }
    },
    [t.common.updatedSuccessfully, t.common.failedToUpdate]
  );

  const checkInstalled = useCallback(async () => {
    try {
      const installed = await gitCheckInstalled();
      setGitInstalled(installed);
    } catch (err) {
      console.error("Failed to check git installation:", err);
      setGitInstalled(false);
    }
  }, []);

  return {
    config,
    loading,
    saving,
    gitInstalled,
    loadConfig,
    saveConfig,
    checkInstalled,
    defaultConfig: DEFAULT_CONFIG,
  };
}
