import { ipc, ipcVoid } from "@/lib/ipc";
import type { ProviderInfo, ModelInfo } from "@/types";

export async function fetchProviders(): Promise<ProviderInfo[]> {
  return ipc<ProviderInfo[]>("provider_list");
}

export async function fetchModels(): Promise<ModelInfo[]> {
  return ipc<ModelInfo[]>("provider_models");
}

export async function testConnection(params: {
  provider: string;
  apiKey: string;
  baseUrl: string;
  model: string;
}): Promise<void> {
  return ipcVoid("provider_test_connection", params);
}

export async function refreshProviders(): Promise<void> {
  return ipcVoid("provider_refresh");
}
