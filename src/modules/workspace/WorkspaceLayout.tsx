import { ScrollArea } from "@/components/ui/scroll-area";

export function WorkspaceLayout({ children }: { children: React.ReactNode }) {
  return (
    <ScrollArea className="h-full">
      <div className="px-6 py-5">{children}</div>
    </ScrollArea>
  );
}
