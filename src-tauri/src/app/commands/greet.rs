use crate::errors::{IpcResponse, AppError};

#[tauri::command]
pub fn greet(name: String) -> Result<IpcResponse<String>, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::invalid_input("Name cannot be empty"));
    }
    if name.len() > 200 {
        return Err(AppError::invalid_input("Name too long (max 200 chars)"));
    }
    let message = format!("Hello, {}! Welcome to Mnemosyne.", name);
    Ok(IpcResponse::ok(message))
}
