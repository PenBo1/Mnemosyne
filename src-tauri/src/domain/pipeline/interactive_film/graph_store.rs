// JSON 持久化。
//
// - load：读文件 → serde_json::from_str → StoryGraph
// - save：serde_json::to_string_pretty → 先写 .tmp 再 rename（原子化）

use std::path::Path;

use crate::shared::error::AppError;

use super::graph_schema::StoryGraph;

/// 从 JSON 文件加载 StoryGraph。
pub fn load_story_graph(path: &Path) -> Result<StoryGraph, AppError> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::file_not_found(path.display().to_string())
        } else {
            AppError::file_read_error(path.display().to_string())
        }
    })?;
    let graph: StoryGraph = serde_json::from_str(&raw)?;
    Ok(graph)
}

/// 将 StoryGraph 以 pretty JSON 写入文件（原子化：先写 .tmp 再 rename）。
pub fn save_story_graph(graph: &StoryGraph, path: &Path) -> Result<(), AppError> {
    let json = serde_json::to_string_pretty(graph)?;
    let tmp_path = path.with_extension("json.tmp");
    if let Err(e) = std::fs::write(&tmp_path, &json) {
        return Err(AppError::file_write_error(format!(
            "{}: {}",
            tmp_path.display(),
            e
        )));
    }
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        // rename 失败时尝试清理临时文件
        let _ = std::fs::remove_file(&tmp_path);
        return Err(AppError::file_write_error(format!(
            "{}: {}",
            path.display(),
            e
        )));
    }
    Ok(())
}
