use crate::errors::{IpcResponse, AppError};

#[tauri::command]
pub fn greet(name: String) -> Result<IpcResponse<String>, AppError> {
    let message = format!("Hello, {}! Welcome to Mnemosyne.", name);
    Ok(IpcResponse::ok(message))
}
