import { ipc } from "@/infrastructure/api";
import type { SandboxStatus, SandboxPolicy } from "@/shared/types";

export async function getSandboxStatus(): Promise<SandboxStatus> {
  return ipc<SandboxStatus>("sandbox_status");
}

export async function validateFile(path: string, isWrite: boolean): Promise<boolean> {
  return ipc<boolean>("sandbox_validate_file", { path, is_write: isWrite });
}

export async function validateCommand(command: string): Promise<boolean> {
  return ipc<boolean>("sandbox_validate_command", { command });
}

export async function validateNetwork(url: string): Promise<boolean> {
  return ipc<boolean>("sandbox_validate_network", { url });
}

export async function getSandboxPolicy(): Promise<SandboxPolicy> {
  return ipc<SandboxPolicy>("sandbox_get_policy");
}
