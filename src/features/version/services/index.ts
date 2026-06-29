import { ipc } from "@/infrastructure/api";
import type { ChapterVersion, LineDiffResult } from "@/shared/types";

// ── Version List & Get ─────────────────────────────────────

export async function listChapterVersions(
  novelId: string,
  chapterNumber: number
): Promise<ChapterVersion[]> {
  return ipc<ChapterVersion[]>("version_list", { novelId, chapterNumber });
}

export async function getChapterVersion(versionId: string): Promise<ChapterVersion | null> {
  return ipc<ChapterVersion | null>("version_get", { versionId });
}

export async function getLatestChapterVersion(
  novelId: string,
  chapterNumber: number
): Promise<ChapterVersion | null> {
  return ipc<ChapterVersion | null>("version_get_latest", { novelId, chapterNumber });
}

// ── Diff Operations ────────────────────────────────────────

export async function computeVersionDiff(
  fromVersionId: string,
  toVersionId: string
): Promise<LineDiffResult> {
  return ipc<LineDiffResult>("version_diff", { fromVersionId, toVersionId });
}

export async function computeLatestDiff(
  novelId: string,
  chapterNumber: number
): Promise<LineDiffResult | null> {
  return ipc<LineDiffResult | null>("version_diff_latest", { novelId, chapterNumber });
}

// ── Version Restore & Save ────────────────────────────────

export async function restoreVersion(
  versionId: string,
  workspaceId: string,
  bookId: string
): Promise<boolean> {
  return ipc<boolean>("version_restore", { versionId, workspaceId, bookId });
}

export async function saveVersion(
  novelId: string,
  chapterNumber: number,
  content: string,
  revisionMode: string,
  revisionReason: string
): Promise<ChapterVersion> {
  return ipc<ChapterVersion>("version_save", {
    novelId,
    chapterNumber,
    content,
    revisionMode,
    revisionReason,
  });
}