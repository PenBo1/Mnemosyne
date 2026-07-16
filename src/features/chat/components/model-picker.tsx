import { memo, useState, useEffect } from "react";
import { toast } from "sonner";
import { CpuIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { useAppDispatch } from "@/lib/app-context";
import { loadSettings, setActiveModel, type AiModelConfig } from "@/services/settings";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

/**
 * 模型选择器：shadcn Select 下拉。
 *
 * - 自包含：从 config.json 读取 ai.models + active_model_id
 * - 切换时写入 config.json（Rust 后端在下次发送消息时读取最新配置）
 * - 无模型时显示「打开设置」按钮，跳转到模型设置页
 */
export const ModelPicker = memo(function ModelPicker() {
  const { t } = useI18n();
  const dispatch = useAppDispatch();
  const [models, setModels] = useState<AiModelConfig[]>([]);
  const [activeModelId, setActiveModelId] = useState<string>("");

  // 加载模型配置
  useEffect(() => {
    let cancelled = false;
    void loadSettings().then((settings) => {
      if (cancelled) return;
      setModels(settings.ai.models);
      setActiveModelId(settings.ai.active_model_id ?? "");
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleValueChange = (value: string) => {
    setActiveModelId(value);
    void setActiveModel(value).catch(() => {
      toast.error(t.common.failedToSave);
    });
  };

  const openModelSettings = () => {
    dispatch({ type: "SET_PAGE", payload: "settings.model" });
  };

  const activeModel = models.find((m) => m.id === activeModelId);

  // 无模型时直接显示「打开设置」按钮，避免空下拉
  if (models.length === 0) {
    return (
      <Button variant="ghost" size="sm" onClick={openModelSettings} className="text-muted-foreground">
        <CpuIcon className="size-4" />
        {t.agentChat.openSettings}
      </Button>
    );
  }

  return (
    <Select value={activeModelId} onValueChange={handleValueChange}>
      <SelectTrigger size="sm" className="min-w-32 max-w-48">
        <CpuIcon className="text-muted-foreground" />
        <SelectValue placeholder={t.agentChat.selectModel}>
          {activeModel?.name ?? ""}
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectLabel>{t.agentChat.modelLabel}</SelectLabel>
          {models.map((m) => (
            <SelectItem key={m.id} value={m.id}>
              {m.name}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
});
