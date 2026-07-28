//! ═══════════════════════════════════════════════════════════════════════════
//! 路径验证 - 文件路径校验
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::Path;

const MAX_PATH_LENGTH: usize = 4096;

pub fn validate_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("路径为空".to_string());
    }
    if path.len() > MAX_PATH_LENGTH {
        return Err("路径过长".to_string());
    }
    if path.contains("..") {
        return Err("路径穿越检测".to_string());
    }
    if path.chars().any(|c| c.is_control()) {
        return Err("路径包含控制字符".to_string());
    }
    Ok(())
}

pub fn validate_path_exists(path: &str) -> Result<(), String> {
    validate_path(path)?;
    if !Path::new(path).exists() {
        return Err(format!("路径不存在: {}", path));
    }
    Ok(())
}

pub fn validate_path_is_absolute(path: &str) -> Result<(), String> {
    validate_path(path)?;
    if !Path::new(path).is_absolute() {
        return Err(format!("路径必须是绝对路径: {}", path));
    }
    Ok(())
}