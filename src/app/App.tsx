import { AppProvider } from "@/lib/app-context";
import { ShortcutProvider } from "@/lib/shortcut-dispatcher";
import { AppLayout } from "@/components/layout/AppLayout";
import { Toaster } from "@/components/ui/sonner";
import { useStartupSettings } from "@/features/settings/hooks";
import { LifecycleManager } from "./LifecycleManager";

function App() {
  useStartupSettings();

  return (
    <AppProvider>
      <ShortcutProvider>
        <LifecycleManager
          onWindowClose={() => {
            console.info("Window closing, cleaning up resources...");
          }}
        />
        <AppLayout />
        <Toaster />
      </ShortcutProvider>
    </AppProvider>
  );
}

export default App;
