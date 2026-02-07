//! Example: Job worker FSM with timeouts

use tokio_fsm::{Transition, fsm};

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
        // Simulate async work
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        Ok(())
    }
}

#[derive(Debug)]
pub enum WorkerError {
    DatabaseError(String),
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for WorkerError {}

#[fsm(initial = "Idle", channel_size = 100)]
impl WorkerFsm {
    type Context = WorkerContext;
    type Error = WorkerError;

    #[event(Job)]
    #[state_timeout(duration = "30s")]
    async fn handle_job(&mut self, job: Job) -> Result<Transition<Working>, Transition<Failed>> {
        self.context
            .db
            .save(&job)
            .await
            .map(|_| Transition::to(Working))
            .map_err(|e| Transition::to_with_data(Failed, e))
    }

    #[event(Done)]
    async fn handle_done(&mut self) -> Transition<Idle> {
        Transition::to(Idle)
    }

    #[on_timeout]
    async fn handle_timeout(&mut self) -> Transition<Failed> {
        Transition::to(Failed)
    }
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
    handle.send(Event::Job(job)).await.unwrap();

    // Wait a bit
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send done event
    handle.send(Event::Done).await.unwrap();

    // Shutdown gracefully
    handle.shutdown_graceful().await;

    // Wait for task
    let _ = task.await_task().await;
}
