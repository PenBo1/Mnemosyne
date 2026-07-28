//! ═══════════════════════════════════════════════════════════════════════════
//! Artifact - Memory artifact 文件管理
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::core::agent::memory::phase1::StageOneOutput;

// ── 常量定义 ────────────────────────────────────────────────────────────────

/// 带元数据的记忆 artifact
///
/// phase1 的 `StageOneOutput` 是纯 model 输出，本结构关联 thread_id 等元数据，
/// 供 artifact 文件管理使用。
#[derive(Debug, Clone)]
pub struct MemoryArtifact {
    pub thread_id: String,
    pub stage_one: StageOneOutput,
}

/// `raw_memories.md` 文件名
pub const RAW_MEMORIES_FILENAME: &str = "raw_memories.md";

/// `rollout_summaries/` 子目录名
pub const ROLLOUT_SUMMARIES_SUBDIR: &str = "rollout_summaries";

/// `phase2_workspace_diff.md` 文件名
pub const WORKSPACE_DIFF_FILENAME: &str = "phase2_workspace_diff.md";

/// workspace diff 最大字节数（4MB）
pub const WORKSPACE_DIFF_MAX_BYTES: usize = 4 * 1024 * 1024;

// ── 路径辅助函数 ────────────────────────────────────────────────────────────
pub fn raw_memories_file(root: &Path) -> PathBuf {
    root.join(RAW_MEMORIES_FILENAME)
}

/// `rollout_summaries/` 目录路径
pub fn rollout_summaries_dir(root: &Path) -> PathBuf {
    root.join(ROLLOUT_SUMMARIES_SUBDIR)
}

/// `phase2_workspace_diff.md` 路径
pub fn workspace_diff_file(root: &Path) -> PathBuf {
    root.join(WORKSPACE_DIFF_FILENAME)
}

/// 确保 memory root 下的目录结构存在
pub fn ensure_layout(root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(root)?;
    std::fs::create_dir_all(rollout_summaries_dir(root))?;
    Ok(())
}

/// 从 DB-backed stage-1 输出重建 `raw_memories.md`
///
/// 格式：
/// ```text
/// # Raw Memories
///
/// Merged stage-1 raw memories (stable ascending thread-id order):
///
/// ## Thread `<thread_id>`
/// updated_at: <rfc3339>
/// cwd: <cwd>
/// rollout_summary_file: <stem>.md
///
/// <raw_memory>
/// ```
pub fn rebuild_raw_memories_file(
    root: &Path,
    memories: &[MemoryArtifact],
    max_raw_memories: usize,
) -> std::io::Result<()> {
    ensure_layout(root)?;

    let retained = &memories[..memories.len().min(max_raw_memories)];
    let mut body = String::from("# Raw Memories\n\n");

    if retained.is_empty() {
        body.push_str("No raw memories yet.\n");
        return std::fs::write(raw_memories_file(root), body);
    }

    body.push_str("Merged stage-1 raw memories (stable ascending thread-id order):\n\n");
    for memory in retained {
        writeln!(body, "## Thread `{}`", memory.thread_id)
            .map_err(fmt_io_error)?;
        writeln!(body, "rollout_summary: {}", memory.stage_one.rollout_summary)
            .map_err(fmt_io_error)?;
        if let Some(slug) = memory.stage_one.rollout_slug.as_deref() {
            writeln!(body, "rollout_slug: {}", slug).map_err(fmt_io_error)?;
        }
        writeln!(body).map_err(fmt_io_error)?;
        body.push_str(memory.stage_one.raw_memory.trim());
        body.push_str("\n\n");
    }

    std::fs::write(raw_memories_file(root), body)
}

/// 同步 rollout summary 文件：保留 keep 集合，删除过期文件
///
/// 以 thread_id 作为文件名 stem（`<thread_id>.md`）。
pub fn sync_rollout_summaries(
    root: &Path,
    memories: &[MemoryArtifact],
    max_raw_memories: usize,
) -> std::io::Result<()> {
    ensure_layout(root)?;

    let retained = &memories[..memories.len().min(max_raw_memories)];
    let keep: HashSet<String> = retained.iter().map(|m| m.thread_id.clone()).collect();

    prune_rollout_summaries(root, &keep)?;

    for memory in retained {
        write_rollout_summary_for_thread(root, memory)?;
    }

    Ok(())
}

/// 删除不在 keep 集合中的 rollout summary 文件
fn prune_rollout_summaries(root: &Path, keep: &HashSet<String>) -> std::io::Result<()> {
    let dir_path = rollout_summaries_dir(root);
    let entries = match std::fs::read_dir(&dir_path) {
        Ok(e) => e,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(stem) = file_name.strip_suffix(".md") else {
            continue;
        };
        if !keep.contains(stem) {
            let _ = std::fs::remove_file(&path);
        }
    }

    Ok(())
}

/// 写单个 thread 的 rollout summary 文件
fn write_rollout_summary_for_thread(
    root: &Path,
    memory: &MemoryArtifact,
) -> std::io::Result<()> {
    let path = rollout_summaries_dir(root).join(format!("{}.md", memory.thread_id));

    let mut body = String::new();
    writeln!(body, "thread_id: {}", memory.thread_id).map_err(fmt_io_error)?;
    writeln!(body, "rollout_summary: {}", memory.stage_one.rollout_summary)
        .map_err(fmt_io_error)?;
    if let Some(slug) = memory.stage_one.rollout_slug.as_deref() {
        writeln!(body, "rollout_slug: {}", slug).map_err(fmt_io_error)?;
    }
    writeln!(body).map_err(fmt_io_error)?;
    body.push_str(&memory.stage_one.rollout_summary);
    body.push('\n');

    std::fs::write(path, body)
}

/// 删除 `phase2_workspace_diff.md`（若存在）
pub fn remove_workspace_diff(root: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(workspace_diff_file(root)) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

/// 将 diff 内容写入 `phase2_workspace_diff.md`，超出 MAX_BYTES 时截断
pub fn write_workspace_diff(root: &Path, diff: &str) -> std::io::Result<()> {
    let mut rendered = String::from(
        "# Memory Workspace Diff\n\n\
         Generated before Phase 2 memory consolidation. Read this file first and do not edit it.\n\n\
         ## Diff\n\n```diff\n",
    );
    append_bounded_diff(&mut rendered, diff);
    rendered.push_str("```\n");
    std::fs::write(workspace_diff_file(root), rendered)
}

/// 追加 diff 内容，超过 MAX_BYTES 时在字符边界截断
fn append_bounded_diff(rendered: &mut String, diff: &str) {
    if diff.len() <= WORKSPACE_DIFF_MAX_BYTES {
        rendered.push_str(diff);
        if !diff.ends_with('\n') {
            rendered.push('\n');
        }
        return;
    }

    let boundary = previous_char_boundary(diff, WORKSPACE_DIFF_MAX_BYTES);
    rendered.push_str(&diff[..boundary]);
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    let _ = writeln!(
        rendered,
        "\n[workspace diff truncated at {} bytes]",
        WORKSPACE_DIFF_MAX_BYTES
    );
}

/// 找到不超过 max_bytes 的最大 UTF-8 字符边界
fn previous_char_boundary(value: &str, max_bytes: usize) -> usize {
    if max_bytes >= value.len() {
        return value.len();
    }
    let mut index = max_bytes;
    while !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn fmt_io_error(err: std::fmt::Error) -> std::io::Error {
    std::io::Error::other(format!("format memory artifact: {err}"))
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_output(thread_id: &str, raw: &str, summary: &str) -> MemoryArtifact {
        MemoryArtifact {
            thread_id: thread_id.to_string(),
            stage_one: StageOneOutput {
                raw_memory: raw.to_string(),
                rollout_summary: summary.to_string(),
                rollout_slug: Some(format!("slug-{}", thread_id)),
            },
        }
    }

    // 测试 1：路径 getter 正确
    #[test]
    fn path_getters_correct() {
        let root = Path::new("/tmp/memories");
        assert_eq!(raw_memories_file(root), root.join("raw_memories.md"));
        assert_eq!(
            rollout_summaries_dir(root),
            root.join("rollout_summaries")
        );
        assert_eq!(
            workspace_diff_file(root),
            root.join("phase2_workspace_diff.md")
        );
    }

    // 测试 2：ensure_layout 创建目录
    #[test]
    fn ensure_layout_creates_dirs() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("memories");
        assert!(!root.exists());

        ensure_layout(&root).unwrap();

        assert!(root.exists());
        assert!(rollout_summaries_dir(&root).exists());
    }

    // 测试 3：rebuild_raw_memories_file 写入正确格式
    #[test]
    fn rebuild_raw_memories_file_writes_correct_format() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("memories");
        let memories = vec![
            sample_output("t1", "memory one", "summary one"),
            sample_output("t2", "memory two", "summary two"),
        ];

        rebuild_raw_memories_file(&root, &memories, 100).unwrap();

        let content = std::fs::read_to_string(raw_memories_file(&root)).unwrap();
        assert!(content.contains("# Raw Memories"));
        assert!(content.contains("## Thread `t1`"));
        assert!(content.contains("memory one"));
        assert!(content.contains("## Thread `t2`"));
        assert!(content.contains("memory two"));
        assert!(content.contains("rollout_slug: slug-t1"));
    }

    // 测试 4：空 memories 写入 "No raw memories yet."
    #[test]
    fn rebuild_raw_memories_file_empty() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("memories");

        rebuild_raw_memories_file(&root, &[], 100).unwrap();

        let content = std::fs::read_to_string(raw_memories_file(&root)).unwrap();
        assert!(content.contains("No raw memories yet."));
    }

    // 测试 5：max_raw_memories 限制保留数量
    #[test]
    fn rebuild_raw_memories_file_respects_max() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("memories");
        let memories = vec![
            sample_output("t1", "m1", "s1"),
            sample_output("t2", "m2", "s2"),
            sample_output("t3", "m3", "s3"),
        ];

        rebuild_raw_memories_file(&root, &memories, 2).unwrap();

        let content = std::fs::read_to_string(raw_memories_file(&root)).unwrap();
        assert!(content.contains("## Thread `t1`"));
        assert!(content.contains("## Thread `t2`"));
        assert!(!content.contains("## Thread `t3`"));
    }

    // 测试 6：sync_rollout_summaries 写入 + 清理过期文件
    #[test]
    fn sync_rollout_summaries_writes_and_prunes() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("memories");
        ensure_layout(&root).unwrap();

        // 先写 t1, t2
        let first = vec![
            sample_output("t1", "m1", "s1"),
            sample_output("t2", "m2", "s2"),
        ];
        sync_rollout_summaries(&root, &first, 100).unwrap();
        assert!(rollout_summaries_dir(&root).join("t1.md").exists());
        assert!(rollout_summaries_dir(&root).join("t2.md").exists());

        // 再写 t2, t3（t1 应被清理）
        let second = vec![
            sample_output("t2", "m2-updated", "s2"),
            sample_output("t3", "m3", "s3"),
        ];
        sync_rollout_summaries(&root, &second, 100).unwrap();
        assert!(
            !rollout_summaries_dir(&root).join("t1.md").exists(),
            "t1.md 应被清理"
        );
        assert!(rollout_summaries_dir(&root).join("t2.md").exists());
        assert!(rollout_summaries_dir(&root).join("t3.md").exists());

        // t2 内容应已更新
        let t2_content =
            std::fs::read_to_string(rollout_summaries_dir(&root).join("t2.md")).unwrap();
        assert!(t2_content.contains("s2"));
    }

    // 测试 7：write_workspace_diff 正常写入
    #[test]
    fn write_workspace_diff_normal() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("memories");
        ensure_layout(&root).unwrap();

        let diff = "--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\n+new\n";
        write_workspace_diff(&root, diff).unwrap();

        let content = std::fs::read_to_string(workspace_diff_file(&root)).unwrap();
        assert!(content.contains("# Memory Workspace Diff"));
        assert!(content.contains("```diff"));
        assert!(content.contains("-old"));
        assert!(content.contains("+new"));
    }

    // 测试 8：write_workspace_diff 超出 MAX_BYTES 时截断
    #[test]
    fn write_workspace_diff_truncates_at_max_bytes() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("memories");
        ensure_layout(&root).unwrap();

        // 构造一个远超 4MB 的 diff
        let big_diff: String = "a".repeat(WORKSPACE_DIFF_MAX_BYTES + 1024);
        write_workspace_diff(&root, &big_diff).unwrap();

        let content = std::fs::read_to_string(workspace_diff_file(&root)).unwrap();
        assert!(
            content.contains("[workspace diff truncated at"),
            "应包含截断标记"
        );
        // 文件总大小应远小于 big_diff 的两倍
        assert!(content.len() < big_diff.len());
    }

    // 测试 9：remove_workspace_diff 幂等
    #[test]
    fn remove_workspace_diff_idempotent() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("memories");
        ensure_layout(&root).unwrap();

        // 文件不存在时不报错
        remove_workspace_diff(&root).unwrap();

        // 写入后删除
        write_workspace_diff(&root, "diff").unwrap();
        assert!(workspace_diff_file(&root).exists());
        remove_workspace_diff(&root).unwrap();
        assert!(!workspace_diff_file(&root).exists());

        // 再次删除（已不存在）不报错
        remove_workspace_diff(&root).unwrap();
    }

    // 测试 10：previous_char_boundary 在 UTF-8 边界截断
    #[test]
    fn previous_char_boundary_handles_multibyte() {
        // 中文每个字符 3 字节
        let value = "你好世界测试"; // 6 chars × 3 bytes = 18 bytes
        let boundary = previous_char_boundary(value, 7);
        // 7 不是字符边界（3,6,9,12,15,18 才是），应回退到 6
        assert_eq!(boundary, 6);
        assert_eq!(&value[..boundary], "你好");

        // 边界对齐时直接返回
        let boundary = previous_char_boundary(value, 6);
        assert_eq!(boundary, 6);
    }
}
