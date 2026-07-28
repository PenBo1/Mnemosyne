//! ═══════════════════════════════════════════════════════════════════════════
//! ID 验证 - 标识符校验
//! ═══════════════════════════════════════════════════════════════════════════

const MAX_ID_LENGTH: usize = 255;

pub fn validate_id(id: &str, name: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err(format!("{} 为空", name));
    }
    if id.len() > MAX_ID_LENGTH {
        return Err(format!("{} 过长", name));
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(format!("{} 包含非法字符", name));
    }
    if id.chars().any(|c| c.is_control()) {
        return Err(format!("{} 包含控制字符", name));
    }
    Ok(())
}

pub fn validate_id_with_prefix(id: &str, prefix: &str, name: &str) -> Result<(), String> {
    validate_id(id, name)?;
    if !id.starts_with(prefix) {
        return Err(format!("{} 必须以前缀 '{}' 开头", name, prefix));
    }
    Ok(())
}