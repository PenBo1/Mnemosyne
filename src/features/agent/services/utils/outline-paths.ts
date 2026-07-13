// Outline Paths —— Phase 5 路径解析（新 prose 文件优先 + legacy 兜底）。
//
// 路径布局：
//   - 新布局：story/outline/story_frame.md + volume_map.md + roles/<tier>/*.md
//   - 旧布局兜底：story_bible.md + volume_outline.md + character_matrix.md
//   - current_state.md 种子占位识别 + 从 roles/种子伏笔派生初始状态
//
// 原实现用 node:fs/promises（readFile/readdir/access）+ node:path/join，
// 这里全部改为 Tauri IPC + joinPath：
//   - readFile → ipc("fs_read_file")，失败返回 fallback
//   - readdir  → ipc("fs_list_directory")，失败返回 []
//   - access   → 用 read 失败判断（不存在即抛错）
//   - join     → joinPath（浏览器端路径拼接）

import { ipc } from "@/services/ipc";
import type { FileEntry } from "@/types/app";
import { joinPath } from "./path-utils";

/** 读取文件内容，失败时返回 fallback（不抛错）。 */
async function readOr(path: string, fallback: string): Promise<string> {
  try {
    return await ipc<string>("fs_read_file", { path });
  } catch {
    return fallback;
  }
}

/** 列出目录条目，失败时返回空数组（不抛错）。 */
async function tryListDirectory(path: string): Promise<ReadonlyArray<FileEntry>> {
  try {
    return await ipc<FileEntry[]>("fs_list_directory", { path });
  } catch {
    return [];
  }
}

/**
 * Detect whether a book uses the Phase 5 new layout (outline/story_frame.md
 * exists on disk). If yes, story_bible.md / book_rules.md are compat shims.
 * If no, those files ARE the authoritative source.
 */
export async function isNewLayoutBook(bookDir: string): Promise<boolean> {
  const content = await readOr(joinPath(bookDir, "story", "outline", "story_frame.md"), "");
  return content.trim().length > 0;
}

/**
 * Whether a book's architect foundation is fully written on disk. A long
 * architect run (especially on a stronger model) can outlive the in-memory
 * create-status tracking — or the server can restart mid-run — leaving the
 * status endpoint with no entry. Checking disk lets create-status answer
 * "ready" truthfully instead of an ambiguous 404 that reads as "failed".
 *
 * "Complete" mirrors the five sections the architect must emit
 * (story_frame / volume_map / book_rules / pending_hooks / roles); a half-built
 * book that is missing any of these is NOT ready.
 */
export async function isBookFoundationComplete(bookDir: string): Promise<boolean> {
  const required = [
    joinPath(bookDir, "book.json"),
    joinPath(bookDir, "story", "outline", "story_frame.md"),
    joinPath(bookDir, "story", "outline", "volume_map.md"),
    joinPath(bookDir, "story", "book_rules.md"),
    joinPath(bookDir, "story", "pending_hooks.md"),
  ];
  for (const path of required) {
    const content = await readOr(path, "");
    if (content.length === 0) return false;
  }
  // Roles must exist via EITHER source the runtime actually reads: a character
  // sheet under the new roles/<tier>/ dir, OR the legacy character_matrix.md
  // (readCharacterContext falls back to it). The architect routinely persists
  // roles to character_matrix.md, so requiring the roles/ dir alone falsely
  // reported a complete book as "missing".
  for (const tier of ["主要角色", "major", "次要角色", "minor"]) {
    const entries = await tryListDirectory(joinPath(bookDir, "story", "roles", tier));
    if (entries.some((file) => file.name.endsWith(".md"))) return true;
  }
  const matrix = await readOr(joinPath(bookDir, "story", "character_matrix.md"), "");
  if (matrix.trim() && hasLegacyCharacterMatrixRoles(matrix)) return true;
  return false;
}

function hasLegacyCharacterMatrixRoles(content: string): boolean {
  const normalized = content
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .filter((line) => !line.includes("兼容提示"))
    .filter((line) => !line.includes("story/roles/"))
    .filter((line) => !/^\(?(?:none|无|暂无)\)?$/i.test(line));

  return normalized.some((line) => {
    const match = /^#{2,}\s+(.+)$/.exec(line);
    if (!match) return false;
    const title = match[1].trim().replace(/[*`#]/g, "");
    return !/^(主要角色|次要角色|major roles?|minor roles?|characters?|角色矩阵)$/i.test(title);
  });
}

/** Read story_frame.md, falling back to legacy story_bible.md. */
export async function readStoryFrame(
  bookDir: string,
  fallbackPlaceholder: string = "",
): Promise<string> {
  const newPath = joinPath(bookDir, "story", "outline", "story_frame.md");
  const legacyPath = joinPath(bookDir, "story", "story_bible.md");

  const newContent = await readOr(newPath, "");
  if (newContent.trim()) return newContent;

  return readOr(legacyPath, fallbackPlaceholder);
}

/** Read volume_map.md, falling back to legacy volume_outline.md. */
export async function readVolumeMap(
  bookDir: string,
  fallbackPlaceholder: string = "",
): Promise<string> {
  const newPath = joinPath(bookDir, "story", "outline", "volume_map.md");
  const legacyPath = joinPath(bookDir, "story", "volume_outline.md");

  const newContent = await readOr(newPath, "");
  if (newContent.trim()) return newContent;

  return readOr(legacyPath, fallbackPlaceholder);
}

/** Read the rhythm principles file (zh or en variant). */
export async function readRhythmPrinciples(bookDir: string): Promise<string> {
  const zhPath = joinPath(bookDir, "story", "outline", "节奏原则.md");
  const enPath = joinPath(bookDir, "story", "outline", "rhythm_principles.md");

  const zh = await readOr(zhPath, "");
  if (zh.trim()) return zh;
  return readOr(enPath, "");
}

export interface RoleCard {
  readonly tier: "major" | "minor";
  readonly name: string;
  readonly content: string;
}

/**
 * Read the roles/ directory. Returns [] when no roles are present (e.g. old
 * books still on character_matrix.md).
 */
export async function readRoleCards(bookDir: string): Promise<ReadonlyArray<RoleCard>> {
  const rolesRoot = joinPath(bookDir, "story", "roles");
  const majorDirZh = joinPath(rolesRoot, "主要角色");
  const minorDirZh = joinPath(rolesRoot, "次要角色");
  const majorDirEn = joinPath(rolesRoot, "major");
  const minorDirEn = joinPath(rolesRoot, "minor");

  const cards: RoleCard[] = [];
  await Promise.all([
    collectRoleDir(majorDirZh, "major", cards),
    collectRoleDir(minorDirZh, "minor", cards),
    collectRoleDir(majorDirEn, "major", cards),
    collectRoleDir(minorDirEn, "minor", cards),
  ]);
  return cards;
}

async function collectRoleDir(
  dir: string,
  tier: "major" | "minor",
  out: RoleCard[],
): Promise<void> {
  const entries = await tryListDirectory(dir);
  const reads = entries
    .filter((entry) => entry.name.endsWith(".md"))
    .map(async (entry) => {
      const content = await readOr(joinPath(dir, entry.name), "");
      if (!content.trim()) return;
      out.push({
        tier,
        name: entry.name.replace(/\.md$/, ""),
        content,
      });
    });
  await Promise.all(reads);
}

/**
 * Render role cards in a format compatible with downstream consumers that
 * previously expected character_matrix.md prose. When no role cards exist,
 * returns the legacy character_matrix.md content or the placeholder.
 */
export async function readCharacterContext(
  bookDir: string,
  fallbackPlaceholder: string = "",
): Promise<string> {
  const cards = await readRoleCards(bookDir);
  if (cards.length > 0) {
    const groups: Record<"major" | "minor", RoleCard[]> = { major: [], minor: [] };
    for (const card of cards) groups[card.tier].push(card);

    const render = (tierCards: RoleCard[], heading: string): string => {
      if (tierCards.length === 0) return "";
      const sections = tierCards.map((card) => `### ${card.name}\n\n${card.content.trim()}`);
      return `## ${heading}\n\n${sections.join("\n\n")}`;
    };

    const blocks = [
      render(groups.major, "主要角色 / Major characters"),
      render(groups.minor, "次要角色 / Minor characters"),
    ].filter(Boolean);

    return blocks.join("\n\n");
  }

  // Fallback: legacy character_matrix.md (may itself be a shim pointer).
  const legacyPath = joinPath(bookDir, "story", "character_matrix.md");
  return readOr(legacyPath, fallbackPlaceholder);
}

// ---------------------------------------------------------------------------
// Phase 5 consolidation: current_state.md initial fallback
//
// After architect consolidation (7→5 sections), current_state.md is seeded
// with a tiny placeholder at book creation. Real content only arrives once
// the consolidator has appended output from chapter 1 onward. Readers that
// previously relied on architect-provided initial state (writer phase-1
// creative prompt, continuity, chapter-analyzer, reviser, composer) should
// substitute a derived initial-state block when the seed placeholder is all
// that's on disk — otherwise the "## 当前状态卡" block in prompts degenerates
// into a meta note about runtime append behaviour.
// ---------------------------------------------------------------------------

/**
 * Marker substring emitted by architect.writeFoundationFiles when seeding
 * current_state.md. Its presence is how readers detect "nothing real yet".
 */
const CURRENT_STATE_SEED_MARKERS = [
  "建书时占位",
  "Seeded at book creation",
];

export function isCurrentStateSeedPlaceholder(raw: string): boolean {
  const trimmed = raw.trim();
  if (!trimmed) return true;
  // Heuristic: short file AND contains one of the seed markers.
  if (trimmed.length > 600) return false;
  return CURRENT_STATE_SEED_MARKERS.some((marker) => trimmed.includes(marker));
}

function extractCurrentStateFromRole(content: string): string | null {
  // Accept both zh (`## 当前现状`) and en (`## Current_State` / `## Current State`).
  const pattern = /^##\s*(?:当前现状|Current[_\s]?State)[^\n]*$/im;
  const match = content.match(pattern);
  if (!match || match.index === undefined) return null;
  const after = content.slice(match.index + match[0].length);
  // Cut at next `## ` heading (same or higher level).
  const nextHeading = after.search(/^##\s/m);
  const raw = nextHeading >= 0 ? after.slice(0, nextHeading) : after;
  const text = raw.trim();
  return text.length > 0 ? text : null;
}

function extractSeedHooksFromPendingHooks(raw: string): string[] {
  if (!raw.trim()) return [];
  const lines = raw.split("\n").map((line) => line.trim()).filter(Boolean);
  const seedRows: string[] = [];
  for (const line of lines) {
    if (!line.startsWith("|")) continue;
    if (/^\|\s*-/.test(line)) continue;
    const cells = line.split("|").slice(1, -1).map((cell) => cell.trim());
    if (cells.length < 2) continue;
    if (cells[0]?.toLowerCase() === "hook_id" || cells[0] === "hookId") continue;
    const startCh = Number.parseInt(cells[1] ?? "", 10);
    if (!Number.isFinite(startCh) || startCh !== 0) continue;
    // cells[2] type, cells[5] expected payoff, last cell notes
    const notes = cells[cells.length - 1] ?? "";
    const summary = [cells[0], cells[2], notes].filter(Boolean).join(" · ");
    if (summary) seedRows.push(summary);
  }
  return seedRows;
}

/**
 * Read current_state.md; when the file is only a seed placeholder (chapter 0,
 * before consolidator has appended anything), derive an initial-state block
 * from roles/*.Current_State + pending_hooks startChapter=0 rows so callers
 * still have substantive content to feed into writer / analyzer prompts.
 */
export async function readCurrentStateWithFallback(
  bookDir: string,
  fallbackPlaceholder: string = "",
): Promise<string> {
  const storyDir = joinPath(bookDir, "story");
  const currentStatePath = joinPath(storyDir, "current_state.md");
  const raw = await readOr(currentStatePath, "");

  if (!isCurrentStateSeedPlaceholder(raw)) {
    return raw;
  }

  const [cards, pendingHooks] = await Promise.all([
    readRoleCards(bookDir),
    readOr(joinPath(storyDir, "pending_hooks.md"), ""),
  ]);

  const roleLines = cards
    .map((card) => {
      const state = extractCurrentStateFromRole(card.content);
      if (!state) return null;
      const tierLabel = card.tier === "major" ? "主要" : "次要";
      return `- ${card.name}（${tierLabel}）：${state.replace(/\s+/g, " ")}`;
    })
    .filter((line): line is string => line !== null);

  const hookLines = extractSeedHooksFromPendingHooks(pendingHooks);

  if (roleLines.length === 0 && hookLines.length === 0) {
    return raw.trim() ? raw : fallbackPlaceholder;
  }

  const parts: string[] = ["# 初始状态（第 0 章，由 roles + 种子伏笔派生）"];
  if (roleLines.length > 0) {
    parts.push("\n## 角色初始位置 / 处境");
    parts.push(...roleLines);
  }
  if (hookLines.length > 0) {
    parts.push("\n## 种子伏笔（startChapter = 0）");
    parts.push(...hookLines.map((line) => `- ${line}`));
  }
  return parts.join("\n");
}
