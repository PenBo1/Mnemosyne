import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { FolderOpenIcon, CopyIcon, CheckIcon } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { SettingsSection, SettingsRow } from "@/features/settings/components/settings-section";
import { getDataDirPath } from "@/features/settings/services";

export function SystemSettings() {
  const { t } = useI18n();
  const [dataDir, setDataDir] = useState<string>("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getDataDirPath()
      .then((path) => {
        if (!cancelled) setDataDir(path);
      })
      .catch((e) => {
        console.error("[system] load data dir path failed", e);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleOpen = async () => {
    if (!dataDir) return;
    try {
      await revealItemInDir(dataDir);
    } catch (e) {
      console.error("[system] reveal data dir failed", e);
      toast.error(t.common.failedToOpen);
    }
  };

  const handleCopy = async () => {
    if (!dataDir) return;
    try {
      await navigator.clipboard.writeText(dataDir);
      setCopied(true);
      toast.success(t.common.copiedToClipboard);
      setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      console.error("[system] copy path failed", e);
      toast.error(t.common.failedToCopy);
    }
  };

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.system}</PageTitle>
          <PageDescription>{t.settings.systemDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      <SettingsSection title={t.settings.field.dataDirPath}>
        <SettingsRow
          label={t.settings.field.dataDirPath}
          description={t.settings.description.dataDirPath}
        >
          <div className="flex items-center gap-1.5">
            <Button
              variant="outline"
              size="sm"
              className="h-8 gap-1.5 px-2 text-xs"
              disabled={!dataDir}
              onClick={() => void handleOpen()}
            >
              <FolderOpenIcon className="size-3.5" />
              {t.settings.openDataDir}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-8 gap-1.5 px-2 text-xs"
              disabled={!dataDir}
              onClick={() => void handleCopy()}
            >
              {copied ? (
                <CheckIcon className="size-3.5" />
              ) : (
                <CopyIcon className="size-3.5" />
              )}
              {copied ? t.common.copied : t.settings.copyPath}
            </Button>
          </div>
        </SettingsRow>
        {dataDir && (
          <div className="px-4 py-3 border-t">
            <p className="font-mono text-xs text-muted-foreground break-all">{dataDir}</p>
          </div>
        )}
      </SettingsSection>
    </PageContainer>
  );
}
