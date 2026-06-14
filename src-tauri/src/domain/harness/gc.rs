use crate::errors::AppError;
use super::types::GcPolicy;

pub struct EntropyManager;

impl EntropyManager {
    pub fn cleanup_stale_snapshots(
        project_root: &std::path::Path,
        policy: &GcPolicy,
    ) -> Result<u32, AppError> {
        let snapshots_dir = project_root.join("story").join("snapshots");
        if !snapshots_dir.exists() {
            return Ok(0);
        }

        let mut cleaned = 0;
        let cutoff_days = policy.stale_snapshot_days as i64;

        for snap_entry in std::fs::read_dir(&snapshots_dir).into_iter().flatten().flatten() {
            let snap_path = snap_entry.path();
            if !snap_path.is_dir() { continue; }

            if let Ok(meta) = std::fs::metadata(&snap_path) {
                if let Ok(modified) = meta.modified() {
                    let age = modified.elapsed().unwrap_or_default();
                    if age.as_secs() > (cutoff_days as u64 * 86400) {
                        if std::fs::remove_dir_all(&snap_path).is_ok() {
                            cleaned += 1;
                        }
                    }
                }
            }
        }

        Ok(cleaned)
    }

    pub fn compact_state(
        project_root: &std::path::Path,
        _book_id: &str,
    ) -> Result<u32, AppError> {
        let state_file = project_root
            .join("story")
            .join("state.json");

        if !state_file.exists() {
            return Ok(0);
        }

        let content = std::fs::read_to_string(&state_file)
            .map_err(|e| AppError::internal(format!("Failed to read state: {}", e)))?;

        let mut state: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse state: {}", e)))?;

        let mut compacted = 0u32;

        if let Some(facts) = state.get_mut("facts").and_then(|v| v.as_array_mut()) {
            let original_len = facts.len();
            let mut seen = std::collections::HashSet::new();
            facts.retain(|f| {
                let key = format!(
                    "{}:{}:{}",
                    f.get("subject").and_then(|v| v.as_str()).unwrap_or(""),
                    f.get("predicate").and_then(|v| v.as_str()).unwrap_or(""),
                    f.get("object").and_then(|v| v.as_str()).unwrap_or("")
                );
                seen.insert(key)
            });
            compacted += (original_len - facts.len()) as u32;
        }

        if let Some(summaries) = state.get_mut("summaries").and_then(|v| v.as_array_mut()) {
            if summaries.len() > 50 {
                let excess = summaries.len() - 50;
                summaries.drain(..excess);
                compacted += excess as u32;
            }
        }

        if compacted > 0 {
            let json = serde_json::to_string_pretty(&state)
                .map_err(|e| AppError::internal(format!("Failed to serialize state: {}", e)))?;
            std::fs::write(&state_file, json)
                .map_err(|e| AppError::internal(format!("Failed to write state: {}", e)))?;
        }

        Ok(compacted)
    }

    pub fn gc_novel(
        project_root: &std::path::Path,
        _book_id: &str,
        policy: &GcPolicy,
    ) -> Result<GcReport, AppError> {
        let mut report = GcReport::default();

        if !project_root.exists() {
            return Ok(report);
        }

        let snapshots_dir = project_root.join("story").join("snapshots");
        if snapshots_dir.exists() {
            let cutoff_days = policy.stale_snapshot_days as i64;
            let mut count = 0u32;

            for entry in std::fs::read_dir(&snapshots_dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if !path.is_dir() { continue; }
                if let Ok(meta) = std::fs::metadata(&path) {
                    if let Ok(modified) = meta.modified() {
                        let age = modified.elapsed().unwrap_or_default();
                        if age.as_secs() > (cutoff_days as u64 * 86400) {
                            if std::fs::remove_dir_all(&path).is_ok() {
                                count += 1;
                            }
                        }
                    }
                }
            }
            report.snapshots_cleaned = count;
        }

        report.state_compacted = Self::compact_state(project_root, _book_id)?;

        Ok(report)
    }
}

#[derive(Debug, Default)]
pub struct GcReport {
    pub snapshots_cleaned: u32,
    pub state_compacted: u32,
}
