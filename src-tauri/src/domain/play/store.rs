// Play 文件系统存储。
//
// 目录结构：
//   <root>/worlds/<world_id>/world.json
//   <root>/worlds/<world_id>/runs/<run_id>/events.jsonl
//                                          /transcript.jsonl
//                                          /current_state.json
//                                          /projections/<name>.json
//                                          /checkpoints/<checkpoint_id>.json
//                                          /graph.db  (PlayDb SQLite)
//
// world_id / run_id / name / checkpoint_id 均经过 validate_id_component 防 `../` 穿越。

use std::path::{Path, PathBuf};

use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::shared::error::AppError;

use super::types::{PlayEvent, PlayWorld};

pub struct PlayStore {
    root: PathBuf,
}

impl PlayStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    // ── 路径构造 ───────────────────────────────────────

    fn worlds_dir(&self) -> PathBuf {
        self.root.join("worlds")
    }

    fn world_dir(&self, world_id: &str) -> Result<PathBuf, AppError> {
        validate_id_component(world_id, "world_id")?;
        Ok(self.worlds_dir().join(world_id))
    }

    fn run_dir(&self, world_id: &str, run_id: &str) -> Result<PathBuf, AppError> {
        validate_id_component(world_id, "world_id")?;
        validate_id_component(run_id, "run_id")?;
        Ok(self.worlds_dir().join(world_id).join("runs").join(run_id))
    }

    /// 某 run 的 graph.db 路径（供命令层构造 PlayRunner）
    pub fn graph_db_path(&self, world_id: &str, run_id: &str) -> Result<PathBuf, AppError> {
        Ok(self.run_dir(world_id, run_id)?.join("graph.db"))
    }

    // ── World CRUD ─────────────────────────────────────

    pub fn create_world(&self, world: &PlayWorld) -> Result<(), AppError> {
        let dir = self.world_dir(&world.world_id)?;
        std::fs::create_dir_all(&dir).map_err(|e| {
            AppError::internal(format!("Failed to create world dir: {}", e))
        })?;
        let path = dir.join("world.json");
        let json = serde_json::to_string_pretty(world)?;
        atomic_write(&path, json.as_bytes())
    }

    pub fn load_world(&self, world_id: &str) -> Result<PlayWorld, AppError> {
        let path = self.world_dir(world_id)?.join("world.json");
        let raw = std::fs::read_to_string(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::file_not_found(path.display().to_string())
            } else {
                AppError::file_read_error(path.display().to_string())
            }
        })?;
        let world: PlayWorld = serde_json::from_str(&raw)?;
        Ok(world)
    }

    pub fn list_worlds(&self) -> Result<Vec<PlayWorld>, AppError> {
        let worlds_dir = self.worlds_dir();
        if !worlds_dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&worlds_dir).map_err(|e| {
            AppError::internal(format!("Failed to read worlds dir: {}", e))
        })? {
            let entry = entry.map_err(|e| AppError::internal(format!("dir entry: {}", e)))?;
            let path = entry.path().join("world.json");
            if !path.exists() {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(raw) => match serde_json::from_str::<PlayWorld>(&raw) {
                    Ok(w) => out.push(w),
                    Err(e) => tracing::warn!(path = %path.display(), error = %e, "skip malformed world.json"),
                },
                Err(e) => tracing::warn!(path = %path.display(), error = %e, "skip unreadable world.json"),
            }
        }
        Ok(out)
    }

    // ── Run 管理 ───────────────────────────────────────

    pub fn ensure_run(&self, world_id: &str, run_id: &str) -> Result<(), AppError> {
        let dir = self.run_dir(world_id, run_id)?;
        std::fs::create_dir_all(dir.join("projections")).map_err(|e| {
            AppError::internal(format!("Failed to create run dir: {}", e))
        })?;
        std::fs::create_dir_all(dir.join("checkpoints")).map_err(|e| {
            AppError::internal(format!("Failed to create checkpoint dir: {}", e))
        })?;
        Ok(())
    }

    // ── 事件流 ─────────────────────────────────────────

    pub fn append_event(
        &self,
        world_id: &str,
        run_id: &str,
        event: &PlayEvent,
    ) -> Result<(), AppError> {
        let path = self.run_dir(world_id, run_id)?.join("events.jsonl");
        let line = serde_json::to_string(event)?;
        append_line(&path, &line)
    }

    pub fn read_events(
        &self,
        world_id: &str,
        run_id: &str,
    ) -> Result<Vec<PlayEvent>, AppError> {
        let path = self.run_dir(world_id, run_id)?.join("events.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let raw = std::fs::read_to_string(&path).map_err(|e| {
            AppError::file_read_error(format!("{}: {}", path.display(), e))
        })?;
        let mut out = Vec::new();
        for (i, line) in raw.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<PlayEvent>(line) {
                Ok(ev) => out.push(ev),
                Err(e) => {
                    tracing::warn!(line = i, error = %e, "skip malformed event line");
                }
            }
        }
        Ok(out)
    }

    // ── Transcript ──────────────────────────────────────

    pub fn append_transcript_turn(
        &self,
        world_id: &str,
        run_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let path = self.run_dir(world_id, run_id)?.join("transcript.jsonl");
        let entry = serde_json::json!({
            "role": role,
            "content": content,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        append_line(&path, &entry.to_string())
    }

    // ── 当前状态 ────────────────────────────────────────

    pub fn save_current_state(
        &self,
        world_id: &str,
        run_id: &str,
        state: &serde_json::Value,
    ) -> Result<(), AppError> {
        let path = self.run_dir(world_id, run_id)?.join("current_state.json");
        let json = serde_json::to_string_pretty(state)?;
        atomic_write(&path, json.as_bytes())
    }

    pub fn load_current_state(
        &self,
        world_id: &str,
        run_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        let path = self.run_dir(world_id, run_id)?.join("current_state.json");
        if !path.exists() {
            return Ok(serde_json::Value::Null);
        }
        let raw = std::fs::read_to_string(&path).map_err(|e| {
            AppError::file_read_error(format!("{}: {}", path.display(), e))
        })?;
        Ok(serde_json::from_str(&raw)?)
    }

    // ── 投影 ───────────────────────────────────────────

    pub fn write_projection(
        &self,
        world_id: &str,
        run_id: &str,
        name: &str,
        content: &str,
    ) -> Result<(), AppError> {
        validate_id_component(name, "projection_name")?;
        let path = self
            .run_dir(world_id, run_id)?
            .join("projections")
            .join(format!("{}.json", name));
        atomic_write(&path, content.as_bytes())
    }

    pub fn read_projection(
        &self,
        world_id: &str,
        run_id: &str,
        name: &str,
    ) -> Result<String, AppError> {
        validate_id_component(name, "projection_name")?;
        let path = self
            .run_dir(world_id, run_id)?
            .join("projections")
            .join(format!("{}.json", name));
        if !path.exists() {
            return Err(AppError::file_not_found(path.display().to_string()));
        }
        std::fs::read_to_string(&path).map_err(|e| {
            AppError::file_read_error(format!("{}: {}", path.display(), e))
        })
    }

    // ── Checkpoint ─────────────────────────────────────

    pub fn save_checkpoint(
        &self,
        world_id: &str,
        run_id: &str,
        checkpoint_id: &str,
        data: &serde_json::Value,
    ) -> Result<(), AppError> {
        validate_id_component(checkpoint_id, "checkpoint_id")?;
        let dir = self
            .run_dir(world_id, run_id)?
            .join("checkpoints");
        std::fs::create_dir_all(&dir).map_err(|e| {
            AppError::internal(format!("Failed to create checkpoint dir: {}", e))
        })?;
        let path = dir.join(format!("{}.json", checkpoint_id));
        let json = serde_json::to_string_pretty(data)?;
        atomic_write(&path, json.as_bytes())
    }

    pub fn load_checkpoint(
        &self,
        world_id: &str,
        run_id: &str,
        checkpoint_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        validate_id_component(checkpoint_id, "checkpoint_id")?;
        let path = self
            .run_dir(world_id, run_id)?
            .join("checkpoints")
            .join(format!("{}.json", checkpoint_id));
        if !path.exists() {
            return Err(AppError::file_not_found(path.display().to_string()));
        }
        let raw = std::fs::read_to_string(&path).map_err(|e| {
            AppError::file_read_error(format!("{}: {}", path.display(), e))
        })?;
        Ok(serde_json::from_str(&raw)?)
    }
}

// ── 文件工具 ──────────────────────────────────────────

fn atomic_write(path: &Path, data: &[u8]) -> Result<(), AppError> {
    let tmp = path.with_extension("tmp");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::internal(format!("Failed to create parent dir: {}", e))
        })?;
    }
    std::fs::write(&tmp, data).map_err(|e| {
        AppError::file_write_error(format!("{}: {}", tmp.display(), e))
    })?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        AppError::file_write_error(format!("{}: {}", path.display(), e))
    })?;
    Ok(())
}

fn append_line(path: &Path, line: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::internal(format!("Failed to create parent dir: {}", e))
        })?;
    }
    let mut content = line.to_string();
    content.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| AppError::file_write_error(format!("{}: {}", path.display(), e)))?;
    use std::io::Write;
    file.write_all(content.as_bytes()).map_err(|e| {
        AppError::file_write_error(format!("{}: {}", path.display(), e))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root() -> PathBuf {
        let dir = std::env::temp_dir().join("mnemosyne_play_store_test");
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn sample_world(id: &str) -> PlayWorld {
        PlayWorld {
            world_id: id.to_string(),
            premise: "测试前提".into(),
            world_contract: serde_json::json!({}),
            visual_contract: None,
            mode: "open".into(),
            language: "zh".into(),
            player_persona: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn world_crud_roundtrip() {
        let root = tmp_root();
        let store = PlayStore::new(root.clone());
        let world = sample_world("w1");
        store.create_world(&world).unwrap();
        let loaded = store.load_world("w1").unwrap();
        assert_eq!(loaded.world_id, "w1");
        let list = store.list_worlds().unwrap();
        assert_eq!(list.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn path_traversal_rejected() {
        let root = tmp_root();
        let store = PlayStore::new(root.clone());
        assert!(store.load_world("../etc").is_err());
        assert!(store.ensure_run("w1", "../x").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
