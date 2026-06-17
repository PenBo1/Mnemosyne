import { useAppState } from "@/lib/app-context";
import { useWorkspaceStore } from "@/stores/workspace";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WorkspaceLayout } from "@/components/layout/WorkspaceLayout";
import { SettingsPage } from "@/pages/SettingsPage";
import { TrendsPage } from "@/pages/TrendsPage";
import { NovelDownloadPage } from "@/pages/NovelDownloadPage";
import { SkillsPage } from "@/pages/SkillsPage";
import { AgentChat } from "@/components/AgentChat";
import { ChapterReader } from "@/pages/ChapterReader";
import { MemoryPage } from "@/pages/MemoryPage";
import { DashboardPage } from "@/pages/DashboardPage";
import { KnowledgePage } from "@/pages/KnowledgePage";
import { OverviewPage } from "@/pages/novel/OverviewPage";
import { CharactersPage } from "@/pages/novel/CharactersPage";
import { WorldbuildingPage } from "@/pages/novel/WorldbuildingPage";
import { PlotPage } from "@/pages/novel/PlotPage";
import { TimelinePage } from "@/pages/novel/TimelinePage";
import { ResearchPage } from "@/pages/novel/ResearchPage";
import { useState } from "react";

export function Router() {
  const { currentPage } = useAppState();
  const { activeWorkspaceId } = useWorkspaceStore();
  const [openNovelId, setOpenNovelId] = useState<string | null>(null);
  const [openNovelTitle, setOpenNovelTitle] = useState<string>("");

  const isWorkspacePage = ["overview", "characters", "worldbuilding", "plot", "timeline", "research"].includes(currentPage);
  const hasActiveWorkspace = activeWorkspaceId !== null;

  if (currentPage === "chat") {
    return <AgentChat />;
  }

  if (currentPage === "memory") {
    return (
      <ScrollArea className="h-full">
        <MemoryPage />
      </ScrollArea>
    );
  }

  if (isWorkspacePage && hasActiveWorkspace) {
    return (
      <WorkspaceLayout>
        {currentPage === "overview" && <OverviewPage />}
        {currentPage === "characters" && <CharactersPage />}
        {currentPage === "worldbuilding" && <WorldbuildingPage />}
        {currentPage === "plot" && <PlotPage />}
        {currentPage === "timeline" && <TimelinePage />}
        {currentPage === "research" && <ResearchPage />}
      </WorkspaceLayout>
    );
  }

  if (openNovelId) {
    return (
      <ChapterReader
        novelId={openNovelId}
        novelTitle={openNovelTitle}
        onBack={() => {
          setOpenNovelId(null);
          setOpenNovelTitle("");
        }}
      />
    );
  }

  return (
    <ScrollArea className="h-full">
      <div className={currentPage === "settings" ? "" : "p-6"}>
        {currentPage === "settings" && <SettingsPage />}
        {currentPage === "trends" && <TrendsPage />}
        {currentPage === "novels" && <NovelDownloadPage />}
        {currentPage === "skills" && <SkillsPage />}
        {currentPage === "dashboard" && <DashboardPage />}
        {currentPage === "knowledge" && <KnowledgePage />}
      </div>
    </ScrollArea>
  );
}
