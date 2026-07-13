// 路径拼接工具 —— 替代 node:path/join（浏览器端无 node:path）。
//
// 简化规则：
// - 把多段路径用平台分隔符拼接，自动去重相邻分隔符
// - 不解析 .. / . （路径合法性由 Rust 端 validate_path 兜底）
// - 自动选择分隔符（Windows 用 \，其他用 /）

/** 拼接路径段。例：joinPath("a/b", "c", "d.md") → "a/b/c/d.md"。 */
export function joinPath(...segments: ReadonlyArray<string>): string {
  if (segments.length === 0) return "";

  // 检测首个绝对路径的分隔符风格（Windows vs POSIX）
  const first = segments[0] ?? "";
  const sep = first.includes("\\") && !first.includes("/") ? "\\" : "/";

  const parts: string[] = [];
  for (const seg of segments) {
    if (!seg) continue;
    // 把所有分隔符统一为当前 sep，再去掉首尾分隔符
    const normalized = seg.replace(/[\\/]+/g, sep).replace(new RegExp(`^${escapeRegex(sep)}+|${escapeRegex(sep)}+$`, "g"), "");
    if (normalized) parts.push(normalized);
  }

  // 保留首段的绝对路径前缀（/ 或 C:\）
  const isAbsolute = /^[\\/]/.test(first) || /^[a-zA-Z]:[\\/]/.test(first);
  const joined = parts.join(sep);
  return isAbsolute ? `${sep}${joined}` : joined;
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
