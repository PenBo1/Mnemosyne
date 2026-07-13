use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::llm::state::LlmState;
use tauri::State;

#[tauri::command]
pub async fn llm_model_list(
    state: State<'_, LlmState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::llm::registry::AiModelConfig>>, AppError> {
    let registry = state.registry.lock().await;
    let models = registry.model_configs().to_vec();
    Ok(IpcResponse::ok(models))
}