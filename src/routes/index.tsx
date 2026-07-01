import { useAppState } from "@/shared/app-context";
import { useWorkspaceStore } from "@/stores/workspace";
import { WorkspaceLayout } from "@/components/layout/WorkspaceLayout";
import { SettingsPage } from "@/pages/SettingsPage";
import { TrendsPage } from "@/pages/TrendsPage";
import { NovelDownloadPage } from "@/pages/NovelDownloadPage";
import { SkillsPage } from "@/pages/SkillsPage";
import { ChapterReader } from "@/pages/ChapterReader";
import { MemoryPage } from "@/pages/MemoryPage";
import { DashboardPage } from "@/pages/DashboardPage";
import { KnowledgePage } from "@/pages/KnowledgePage";
import { WikiPage } from "@/pages/WikiPage";
import { VersionPage } from "@/pages/VersionPage";
import { GitPage } from "@/pages/GitPage";
import ChatPage from "@/pages/ChatPage";
import LoopPage from "@/pages/LoopPage";
import { OverviewPage } from "@/pages/novel/OverviewPage";
import { CharactersPage } from "@/pages/novel/CharactersPage";
import { WorldbuildingPage } from "@/pages/novel/WorldbuildingPage";
import { PlotPage } from "@/pages/novel/PlotPage";
import { TimelinePage } from "@/pages/novel/TimelinePage";
import { ResearchPage } from "@/pages/novel/ResearchPage";
import { useState } from "react";

export function Router() {
  const { currentPage } = useAppState();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const [openNovelId, setOpenNovelId] = useState<string | null>(null);
  const [openNovelTitle, setOpenNovelTitle] = useState<string>("");

  const isWorkspacePage = ["overview", "characters", "worldbuilding", "plot", "timeline", "research"].includes(currentPage);
  const hasActiveWorkspace = activeWorkspaceId !== null;

  if (currentPage === "chat" || currentPage === "main-agent") {
    return <ChatPage />;
  }

  if (currentPage === "memory") {
    return <MemoryPage />;
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

  // Pages with PageContainer handle their own padding & scrolling
  if (currentPage === "settings") {
    return <SettingsPage />;
  }

  if (["trends", "novels", "skills", "dashboard", "knowledge", "wiki", "version", "git"].includes(currentPage)) {
    return (
      <>
        {currentPage === "trends" && <TrendsPage />}
        {currentPage === "novels" && <NovelDownloadPage />}
        {currentPage === "skills" && <SkillsPage />}
        {currentPage === "dashboard" && <DashboardPage />}
        {currentPage === "knowledge" && <KnowledgePage />}
        {currentPage === "wiki" && <WikiPage />}
        {currentPage === "version" && <VersionPage novelId="default" />}
        {currentPage === "git" && <GitPage />}
      </>
    );
  }

  // LoopPage manages its own layout
  return (
    <>
      {currentPage === "loops" && <LoopPage />}
    </>
  );
}
