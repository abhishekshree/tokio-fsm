//! Example: Job worker FSM

use tokio::time::{Duration, sleep};
use tokio_fsm::fsm;

#[derive(Debug, Clone)]
pub struct Job {
    pub id: u64,
    pub data: String,
}

#[derive(Debug)]
pub struct WorkerContext {
    pub db: Database,
}

#[derive(Debug)]
pub struct Database;

impl Database {
    async fn save(&self, _job: &Job) -> Result<(), WorkerError> {
        sleep(Duration::from_millis(10)).await;
        Ok(())
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum WorkerError {
    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[fsm(initial = Idle, channel_size = 100)]
impl WorkerFsm {
    type Context = WorkerContext;
    type Error = WorkerError;

    #[on(state = Idle, event = Job, next = Working)]
    async fn handle_job(&mut self, job: Job) -> Result<(), WorkerError> {
        self.context.db.save(&job).await
    }

    #[on(state = Working, event = Done, next = Idle)]
    async fn handle_done(&mut self) {}
}

#[tokio::main]
async fn main() {
    let context = WorkerContext { db: Database };
    let (handle, task) = WorkerFsm::spawn(context);

    // Send a job
    let job = Job {
        id: 1,
        data: "test".to_string(),
    };
    handle.apply(WorkerFsmEvent::Job(job)).await.unwrap();

    // Wait a bit
    sleep(Duration::from_millis(100)).await;

    // Send done event
    handle.apply(WorkerFsmEvent::Done).await.unwrap();

    // Shutdown cooperatively and wait for the final context
    handle.shutdown();
    let _ = task.await;
}
