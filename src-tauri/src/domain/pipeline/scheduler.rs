use std::sync::Arc;
use tokio::sync::Mutex;
use crate::errors::AppError;
use super::runner::PipelineRunner;

pub struct SchedulerConfig {
    pub write_interval_secs: u64,
    pub radar_interval_secs: u64,
    pub max_concurrent_books: usize,
    pub chapters_per_cycle: u32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            write_interval_secs: 3600, // 1 hour
            radar_interval_secs: 86400, // 24 hours
            max_concurrent_books: 1,
            chapters_per_cycle: 1,
        }
    }
}

pub struct Scheduler {
    pipeline: PipelineRunner,
    #[allow(dead_code)]
    config: SchedulerConfig,
    running: Arc<Mutex<bool>>,
}

impl Scheduler {
    pub fn new(pipeline: PipelineRunner, config: SchedulerConfig) -> Self {
        Self {
            pipeline,
            config,
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn start(&self) -> Result<(), AppError> {
        let mut running = self.running.lock().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        tracing::info!("Scheduler started");
        Ok(())
    }

    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
        tracing::info!("Scheduler stopped");
    }

    pub async fn trigger_write_cycle(&self, book_ids: &[String]) -> Result<(), AppError> {
        for book_id in book_ids {
            match self.pipeline.write_next_chapter(book_id, None).await {
                Ok(result) => {
                    tracing::info!(
                        book_id = %book_id,
                        chapter = result.chapter_number,
                        word_count = result.word_count,
                        "Chapter written"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        book_id = %book_id,
                        error = %e,
                        "Failed to write chapter"
                    );
                }
            }
        }
        Ok(())
    }
}
