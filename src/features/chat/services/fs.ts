import { ipc } from "@/infrastructure/api";
import type { FileEntry } from "@/shared/types";

/**
 * File system IPC commands — reads files and lists directories on the host machine.
 * These map to Rust-side fs_read_file and fs_list_directory commands.
 */
export async function readFile(path: string): Promise<string> {
  return ipc<string>("fs_read_file", { path });
}

export async function listDirectory(path: string): Promise<FileEntry[]> {
  return ipc<FileEntry[]>("fs_list_directory", { path });
}
