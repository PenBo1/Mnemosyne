import { Fragment } from "react";
import { WorkspaceLayout } from "@/features/workspace/WorkspaceLayout";

export const layoutRegistry: Record<string, React.ComponentType<{ children: React.ReactNode }>> = {
  default: Fragment,
  workspace: WorkspaceLayout,
};