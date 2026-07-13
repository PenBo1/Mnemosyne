// Rules reader —— Tauri IPC 版。
//
// 用 fs_read_file IPC 替代 node:fs/promises.readFile，
// 内置 genres 用 Vite import.meta.glob 在构建时嵌入（无需运行时 fs）。
//
// 查找顺序：
//   1. 项目级：{projectRoot}/genres/{genreId}.md
//   2. 内置：src/modules/chat/lib/agent/agents/genres/{genreId}.md（构建时 glob）
//   3. Fallback：内置 other.md

import { ipc } from "@/services/ipc";
import { parseGenreProfile, type ParsedGenreProfile } from "@/types/genre-profile";
import {
  parseBookRules,
  type ParsedBookRules,
} from "@/types/book-rules";
import { BookConfigSchema } from "@/types/book";
import { joinPath } from "../utils/path-utils";

// Vite 构建时嵌入内置 genres。 eager: 同步加载；query: "?raw"：原始字符串。
const BUILTIN_GENRES: Record<string, string> = import.meta.glob("../prompts/genres/*.md", {
  query: "?raw",
  import: "default",
  eager: true,
});

/** 内置 genre id → markdown 内容。 */
function getBuiltinGenreRaw(genreId: string): string | null {
  // glob 的 key 是相对路径，如 "./genres/xuanhuan.md"
  for (const [path, content] of Object.entries(BUILTIN_GENRES)) {
    const match = path.match(/\/([^/]+)\.md$/);
    if (match && match[1] === genreId) {
      return content;
    }
  }
  return null;
}

/** 列出所有内置 genre id。 */
function listBuiltinGenreIds(): string[] {
  const ids: string[] = [];
  for (const path of Object.keys(BUILTIN_GENRES)) {
    const match = path.match(/\/([^/]+)\.md$/);
    if (match) ids.push(match[1]!);
  }
  return ids;
}

/** Tauri IPC 读文件，失败返回 null（不抛错，供 caller 走 fallback）。 */
async function tryReadFile(path: string): Promise<string | null> {
  try {
    return await ipc<string>("fs_read_file", { path });
  } catch {
    return null;
  }
}

/**
 * 加载 genre profile。查找顺序：
 *   1. 项目级：{projectRoot}/genres/{genreId}.md
 *   2. 内置：构建时嵌入的 genres/{genreId}.md
 *   3. Fallback：内置 other.md
 *
 * fail-loud：三级查找都失败时抛错，不静默降级。
 */
export async function readGenreProfile(
  projectRoot: string,
  genreId: string,
): Promise<ParsedGenreProfile> {
  // 1. 项目级
  if (projectRoot) {
    const projectPath = joinPath(projectRoot, "genres", `${genreId}.md`);
    const raw = await tryReadFile(projectPath);
    if (raw) {
      return parseGenreProfile(raw);
    }
  }

  // 2. 内置
  const builtinRaw = getBuiltinGenreRaw(genreId);
  if (builtinRaw) {
    return parseGenreProfile(builtinRaw);
  }

  // 3. Fallback
  const fallbackRaw = getBuiltinGenreRaw("other");
  if (!fallbackRaw) {
    throw new Error(
      `Genre profile not found for "${genreId}" and fallback "other.md" is missing`,
    );
  }

  return parseGenreProfile(fallbackRaw);
}

/** 列出所有可用 genre（项目级 + 内置，去重）。 */
export async function listAvailableGenres(
  projectRoot: string,
): Promise<ReadonlyArray<{ readonly id: string; readonly name: string; readonly source: "project" | "builtin" }>> {
  const results = new Map<string, { id: string; name: string; source: "project" | "builtin" }>();

  // 内置优先（项目级会覆盖）
  for (const id of listBuiltinGenreIds()) {
    const raw = getBuiltinGenreRaw(id);
    if (!raw) continue;
    try {
      const parsed = parseGenreProfile(raw);
      results.set(id, { id, name: parsed.profile.name, source: "builtin" });
    } catch {
      // 内置文件解析失败 —— 不应发生，但 fail-loud 会破坏 listAvailableGenres
      // 的语义（应为"列出可用"，不是"校验全部"）。跳过这一个，让 caller 看到其他可用项。
    }
  }

  // 项目级覆盖
  if (projectRoot) {
    const projectDir = joinPath(projectRoot, "genres");
    try {
      const entries = await ipc<ReadonlyArray<{ name: string; path: string; isDir: boolean; extension: string | null; size: number }>>(
        "fs_list_directory",
        { path: projectDir },
      );
      for (const entry of entries) {
        if (!entry.name.endsWith(".md")) continue;
        const id = entry.name.replace(/\.md$/, "");
        const raw = await tryReadFile(joinPath(projectDir, entry.name));
        if (!raw) continue;
        try {
          const parsed = parseGenreProfile(raw);
          results.set(id, { id, name: parsed.profile.name, source: "project" });
        } catch {
          // 项目级文件解析失败也跳过（用户可能正在编辑中）
        }
      }
    } catch {
      // 项目无 genres 目录 —— 忽略
    }
  }

  return [...results.values()].sort((a, b) => a.id.localeCompare(b.id));
}

/**
 * 加载 book_rules.md。新格式是普通 Markdown（parseBookRules 提取结构化字段）。
 * 旧版 Phase 5 数据可能有 story_frame.md 的 YAML frontmatter —— Mnemosyne
 * 是新项目无 legacy，不支持该 fallback，直接返回 null 让 caller 处理。
 */
export async function readBookRules(bookDir: string): Promise<ParsedBookRules | null> {
  const rulesRaw = await tryReadFile(joinPath(bookDir, "story", "book_rules.md"));
  if (!rulesRaw) return null;

  const parsed = parseBookRules(rulesRaw);
  return parsed; // 可能是 null（shim 检测）
}

/** 读取 book.json 的 language 字段。 */
export async function readBookLanguage(bookDir: string): Promise<"zh" | "en" | undefined> {
  const raw = await tryReadFile(joinPath(bookDir, "book.json"));
  if (!raw) return undefined;

  try {
    const parsed = BookConfigSchema.pick({ language: true }).safeParse(JSON.parse(raw));
    return parsed.success ? parsed.data.language : undefined;
  } catch {
    return undefined;
  }
}
