// 检测历史文件持久化 —— 按 book_id 存储到 detection_dir/<book_id>.json。
//
// 文件格式: Vec<DetectionHistoryEntry>(JSON pretty-print)。
// 记录检测条目时自动计算 attempt(同 chapter 现有条目数 + 1)。

use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

use super::types::DetectionHistoryEntry;

/// 加载某 book 的检测历史(文件不存在时返回空)。
pub fn load_history(data_dir: &DataDir, book_id: &str) -> Result<Vec<DetectionHistoryEntry>, AppError> {
    let path = data_dir.detection_dir().join(format!("{}.json", book_id));
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| AppError::file_read_error(format!("detection history (book {}): {}", book_id, e)))?;
    let entries: Vec<DetectionHistoryEntry> = serde_json::from_str(&raw)
        .map_err(|e| AppError::invalid_format(format!("Detection history corrupt: {}", e)))?;
    Ok(entries)
}

/// 追加一条检测历史并落盘,返回写入的条目(含已计算的 attempt)。
pub fn record_entry(
    data_dir: &DataDir,
    book_id: &str,
    chapter_number: u32,
    action: &str,
    score: f64,
    provider: &str,
    detected_at: &str,
) -> Result<DetectionHistoryEntry, AppError> {
    let mut history = load_history(data_dir, book_id)?;
    let attempt = (history
        .iter()
        .filter(|e| e.chapter_number == chapter_number)
        .count() as u32)
        + 1;
    let entry = DetectionHistoryEntry {
        book_id: book_id.to_string(),
        chapter_number,
        action: action.to_string(),
        attempt,
        score,
        provider: provider.to_string(),
        detected_at: detected_at.to_string(),
    };
    history.push(entry.clone());

    let dir = data_dir.detection_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::internal(format!("Failed to create detection dir: {}", e)))?;
    let path = dir.join(format!("{}.json", book_id));
    let json = serde_json::to_string_pretty(&history)
        .map_err(|e| AppError::internal(format!("History serialize failed: {}", e)))?;
    std::fs::write(&path, json)
        .map_err(|_e| AppError::file_write_error(format!("detection history (book {})", book_id)))?;
    Ok(entry)
}

/// 删除某 book 的检测历史。返回是否删除了文件。
pub fn delete_history(data_dir: &DataDir, book_id: &str) -> Result<bool, AppError> {
    let path = data_dir.detection_dir().join(format!("{}.json", book_id));
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|_e| AppError::file_write_error(format!("detection history (book {})", book_id)))?;
        Ok(true)
    } else {
        Ok(false)
    }
}
