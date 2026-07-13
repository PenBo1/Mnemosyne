
use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

pub fn initialize_app_business_state(data_dir: &DataDir) -> Result<(), AppError> {
    tracing::info!("Initializing app business state");

    let sources_json = include_str!("../../resources/novel_sources.json");
    let sources_path = data_dir.root().join("novel_sources.json");

    if !sources_path.exists() {
        std::fs::write(&sources_path, sources_json)
            .map_err(|_e| AppError::file_write_error(sources_path.display().to_string()))?;
        tracing::info!(path = %sources_path.display(), "Extracted built-in novel sources");
    }

    tracing::info!("App business state initialized");
    Ok(())
}