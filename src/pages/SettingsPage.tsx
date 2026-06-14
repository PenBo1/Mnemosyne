import { useAppState } from "@/lib/app-context";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import { GeneralSettings } from "./settings/GeneralSettings";
import { ModelSettings } from "./settings/ModelSettings";
import { PromptsSettings } from "./settings/PromptsSettings";
import { AgentsSettings } from "./settings/AgentsSettings";
import { AuditSettings } from "./settings/AuditSettings";
import { SystemSettings } from "./settings/SystemSettings";

export function SettingsPage() {
  const { settingsTab } = useAppState();

  return (
    <Tabs value={settingsTab} className="h-full">
      <ScrollArea className="h-full">
        <div className="px-6 py-5">
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
          <TabsContent value="audit" className="mt-0">
            <AuditSettings />
          </TabsContent>
          <TabsContent value="system" className="mt-0">
            <SystemSettings />
          </TabsContent>
        </div>
      </ScrollArea>
    </Tabs>
  );
}
