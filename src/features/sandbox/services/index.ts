import { ipc } from "@/services/ipc";
import type { SandboxStatus } from "@/features/sandbox/types";

export async function getSandboxStatus(): Promise<SandboxStatus> {
  return ipc<SandboxStatus>("sandbox_status");
}
