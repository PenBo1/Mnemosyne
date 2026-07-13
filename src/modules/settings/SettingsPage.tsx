import { useAppState } from "@/shared/app-context";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import { GeneralSettings } from "./settings/GeneralSettings";
import { ModelSettings } from "./settings/ModelSettings";
import { PromptsSettings } from "./settings/PromptsSettings";
import { AgentsSettings } from "./settings/AgentsSettings";
import { BookSourcesSettings } from "./settings/BookSourcesSettings";
import { AuditSettings } from "./settings/AuditSettings";
import { SystemSettings } from "./settings/SystemSettings";
import { GitSettings } from "./settings/GitSettings";

export function SettingsPage() {
  const { settingsTab } = useAppState();

  return (
    <Tabs value={settingsTab} className="h-full">
      <ScrollArea className="h-full">
        <TabsContent value="general" className="mt-0">
          <GeneralSettings />
        </TabsContent>
        <TabsContent value="model" className="mt-0">
          <ModelSettings />
        </TabsContent>
        <TabsContent value="prompts" className="mt-0">
          <PromptsSettings />
        </TabsContent>
        <TabsContent value="agents" className="mt-0">
          <AgentsSettings />
        </TabsContent>
        <TabsContent value="bookSources" className="mt-0">
          <BookSourcesSettings />
        </TabsContent>
        <TabsContent value="audit" className="mt-0">
          <AuditSettings />
        </TabsContent>
        <TabsContent value="system" className="mt-0">
          <SystemSettings />
        </TabsContent>
        <TabsContent value="git" className="mt-0">
          <GitSettings />
        </TabsContent>
      </ScrollArea>
    </Tabs>
  );
}
