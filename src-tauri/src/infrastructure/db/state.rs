
use super::connection::Database;
use crate::infrastructure::fs::data_dir::DataDir;

pub struct DbState {
    pub db: Database,
    pub data_dir: DataDir,
}

impl DbState {
    pub fn new(data_dir: DataDir) -> Self {
        let db_path = data_dir.state_db_path();
        let database = Database::new(db_path.to_str().unwrap())
            .expect("failed to open state database");
        tracing::info!("State database initialized (migrations applied)");
        Self { db: database, data_dir }
    }
}