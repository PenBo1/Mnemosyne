import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import { getCustomInstructions, setCustomInstructions } from "@/services/settings";

/** 全局自定义指令：追加到所有智能体系统提示词末尾 */
export function useCustomInstructions() {
  const { t } = useI18n();
  const [value, setValue] = useState("");
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void getCustomInstructions().then((v) => {
      if (cancelled) return;
      setValue(v);
      setLoaded(true);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const save = useCallback(async (text: string) => {
    try {
      await setCustomInstructions(text);
      setValue(text);
      toast.success(t.common.updatedSuccessfully);
    } catch (e) {
      console.error("[custom-instructions] save failed", e);
      toast.error(t.common.failedToSave);
    }
  }, []);

  return { value, loaded, save };
}
