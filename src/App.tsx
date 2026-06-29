import { AppProvider } from "@/shared/app-context";
import { AppLayout } from "@/components/layout/AppLayout";
import { Toaster } from "@/components/ui/sonner";

function App() {
  return (
    <AppProvider>
      <AppLayout />
      <Toaster />
    </AppProvider>
  );
}

export default App;
