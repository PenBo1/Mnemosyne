use tauri::State;
use crate::shared::error::{IpcResponse, AppError};
use crate::infrastructure::db::state::DbState;
use crate::domain::user::store::UserProfileStore;
use crate::domain::user::types::UserProfile;

#[tauri::command]
pub async fn user_get_profile(
    state: State<'_, DbState>,
) -> Result<IpcResponse<UserProfile>, AppError> {
    let store = UserProfileStore::new(state.data_dir.root());
    let profile = store.get().clone();
    Ok(IpcResponse::ok(profile))
}

#[tauri::command]
pub async fn user_update_profile(
    state: State<'_, DbState>,
    profile: UserProfile,
) -> Result<IpcResponse<UserProfile>, AppError> {
    let mut store = UserProfileStore::new(state.data_dir.root());
    store.update(profile)?;
    Ok(IpcResponse::ok(store.get().clone()))
}