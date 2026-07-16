
use super::connection::Database;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

pub struct DbState {
    pub db: Database,
    pub data_dir: DataDir,
}

impl DbState {
    pub fn new(data_dir: DataDir) -> Result<Self, AppError> {
        let db_path = data_dir.state_db_path();
        let db_path_str = db_path.to_str()
            .ok_or_else(|| AppError::internal("State database path is not valid UTF-8"))?;
        let database = Database::new(db_path_str)?;
        tracing::info!(path = %db_path.display(), "State database initialized (migrations applied)");
        Ok(Self { db: database, data_dir })
    }
}