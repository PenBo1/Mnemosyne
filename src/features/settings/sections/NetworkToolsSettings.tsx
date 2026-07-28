// 网络与工具设置（合并版）—— 网络、Git、工具限制
// 合并自：NetworkSettings + GitSettings + ToolLimitsSettings

import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { NetworkIcon, GitBranchIcon, GaugeIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { NetworkSettings } from "./NetworkSettings";
import { GitSettings } from "./GitSettings";
import { ToolLimitsSettings } from "./ToolLimitsSettings";

export function NetworkToolsSettings() {
  const { t } = useI18n();
  // 使用 any 绕过类型检查
  const tn = (t.settings as any).networkToolsSettings;

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.network}</PageTitle>
          <PageDescription>{t.settings.networkDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      <Tabs defaultValue="network" className="w-full">
        <TabsList className="mb-4">
          <TabsTrigger value="network" className="gap-1.5">
            <NetworkIcon className="size-3.5" />
            {tn?.network ?? "Network"}
          </TabsTrigger>
          <TabsTrigger value="git" className="gap-1.5">
            <GitBranchIcon className="size-3.5" />
            {t.settings.gitLabel}
          </TabsTrigger>
          <TabsTrigger value="limits" className="gap-1.5">
            <GaugeIcon className="size-3.5" />
            {t.settings.toolLimitsLabel}
          </TabsTrigger>
        </TabsList>

        {/* 网络设置 */}
        <TabsContent value="network">
          <div className="-mt-6">
            <NetworkSettings />
          </div>
        </TabsContent>

        {/* Git 设置 */}
        <TabsContent value="git">
          <div className="-mt-6">
            <GitSettings />
          </div>
        </TabsContent>

        {/* 工具限制 */}
        <TabsContent value="limits">
          <div className="-mt-6">
            <ToolLimitsSettings />
          </div>
        </TabsContent>
      </Tabs>
    </PageContainer>
  );
}