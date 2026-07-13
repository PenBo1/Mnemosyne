import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import {
  getNetworkSettings,
  saveNetworkSettings,
} from "@/services/settings";
import type { NetworkSettings as NetworkSettingsType } from "@/services/settings";

export function useNetworkSettings() {
  const { t } = useI18n();
  const [proxyEnabled, setProxyEnabled] = useState(false);
  const [proxyHost, setProxyHost] = useState("");
  const [proxyPort, setProxyPort] = useState("7890");
  const [proxyUsername, setProxyUsername] = useState("");
  const [proxyPassword, setProxyPassword] = useState("");
  const [timeout, setTimeoutValue] = useState("30");

  // 初次加载：读取已持久化的设置（带取消标志，避免卸载后 setState）
  useEffect(() => {
    let cancelled = false;
    getNetworkSettings().then((settings) => {
      if (cancelled) return;
      setProxyEnabled(settings.proxyEnabled);
      setProxyHost(settings.proxyHost);
      setProxyPort(settings.proxyPort);
      setProxyUsername(settings.proxyUsername);
      setProxyPassword(settings.proxyPassword);
      setTimeoutValue(settings.timeout);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const saveSettings = useCallback(async (newSettings: Partial<NetworkSettingsType>) => {
    try {
      await saveNetworkSettings(newSettings);
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToSave);
    }
  }, [t]);

  // toggleProxy 立即持久化（开关切换是离散动作，无节流问题）
  const toggleProxy = useCallback((enabled: boolean) => {
    setProxyEnabled(enabled);
    void saveSettings({ proxyEnabled: enabled });
  }, [saveSettings]);

  // 失焦时统一保存 proxy 相关字段 + timeout
  // 原因：onChange 每键触发 IO 会过度写盘；改为 onBlur 在输入结束时一次性保存
  const saveProxySettings = useCallback(() => {
    void saveSettings({
      proxyHost,
      proxyPort,
      proxyUsername,
      proxyPassword,
      timeout,
    });
  }, [proxyHost, proxyPort, proxyUsername, proxyPassword, timeout, saveSettings]);

  return {
    proxyEnabled,
    proxyHost,
    proxyPort,
    proxyUsername,
    proxyPassword,
    timeout,
    toggleProxy,
    setProxyHost,
    setProxyPort,
    setProxyUsername,
    setProxyPassword,
    setTimeout: setTimeoutValue,
    saveProxySettings,
  };
}
