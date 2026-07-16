use std::sync::OnceLock;
use tauri::State;
use tokio::sync::Mutex;
use crate::shared::error::{IpcResponse, AppError};
use crate::infrastructure::db::state::DbState;
use crate::domain::user::store::UserProfileStore;
use crate::domain::user::types::UserProfile;

/// 全局串行化 user_profile.json 读改写，防止 TOCTOU 竞态。
fn user_profile_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[tauri::command]
pub async fn user_get_profile(
    state: State<'_, DbState>,
) -> Result<IpcResponse<UserProfile>, AppError> {
    let _guard = user_profile_lock().lock().await;
    let store = UserProfileStore::new(state.data_dir.root());
    let profile = store.get().clone();
    Ok(IpcResponse::ok(profile))
}

#[tauri::command]
pub async fn user_update_profile(
    state: State<'_, DbState>,
    profile: UserProfile,
) -> Result<IpcResponse<UserProfile>, AppError> {
    let _guard = user_profile_lock().lock().await;
    let mut store = UserProfileStore::new(state.data_dir.root());
    store.update(profile)?;
    Ok(IpcResponse::ok(store.get().clone()))
}