use std::time::Duration;

use tokio_fsm::{Transition, fsm};

#[derive(Debug, Default)]
pub struct TestContext {
    pub transition_count: usize,
    pub job_data: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Internal error: {0}")]
    Internal(String),
}

#[fsm(initial = Idle, channel_size = 32)]
impl IntegrationFsm {
    type Context = TestContext;
    type Error = TestError;

    #[on(state = Idle, event = Start)]
    #[state_timeout(duration = "100ms")]
    async fn handle_start(&mut self) -> Transition<Pending> {
        self.context.transition_count += 1;
        Transition::to(Pending)
    }

    #[on(state = Pending, event = Process)]
    #[on(state = Active, event = Process)]
    #[state_timeout(duration = "100ms")]
    async fn handle_process(&mut self, data: String) -> Transition<Active> {
        self.context.transition_count += 1;
        self.context.job_data.push(data);
        Transition::to(Active)
    }

    #[on(state = Active, event = Finish)]
    async fn handle_finish(&mut self) -> Transition<Done> {
        self.context.transition_count += 1;
        Transition::to(Done)
    }

    #[on_timeout]
    async fn handle_timeout(&mut self) -> Transition<Failed> {
        self.context.transition_count += 1;
        Transition::to(Failed)
    }
}

#[tokio::test]
async fn test_fsm_full_lifecycle() {
    let context = TestContext::default();
    let (handle, task) = IntegrationFsm::spawn(context);

    assert_eq!(handle.current_state(), IntegrationFsmState::Idle);

    // Idle -> Pending
    handle.send(IntegrationFsmEvent::Start).await.unwrap();
    handle
        .wait_for_state(IntegrationFsmState::Pending)
        .await
        .unwrap();

    // Pending -> Active (with data)
    handle
        .send(IntegrationFsmEvent::Process("task1".to_string()))
        .await
        .unwrap();
    handle
        .wait_for_state(IntegrationFsmState::Active)
        .await
        .unwrap();

    // Active -> Done
    handle.send(IntegrationFsmEvent::Finish).await.unwrap();
    handle
        .wait_for_state(IntegrationFsmState::Done)
        .await
        .unwrap();

    // Shutdown and verify context
    handle.shutdown_graceful();
    let final_context = task.await.unwrap();

    assert_eq!(final_context.transition_count, 3);
    assert_eq!(final_context.job_data, vec!["task1"]);
}

#[tokio::test]
async fn test_fsm_timeout() {
    let context = TestContext::default();
    let (handle, task) = IntegrationFsm::spawn(context);

    // Idle -> Pending
    handle.send(IntegrationFsmEvent::Start).await.unwrap();
    handle
        .wait_for_state(IntegrationFsmState::Pending)
        .await
        .unwrap();

    // Wait for timeout (100ms)
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(handle.current_state(), IntegrationFsmState::Failed);

    handle.shutdown_immediate();
    let final_context = task.await.unwrap();
    assert_eq!(final_context.transition_count, 2); // Start + Timeout
}

#[tokio::test]
async fn test_fsm_graceful_shutdown() {
    let context = TestContext::default();
    let (handle, task) = IntegrationFsm::spawn(context);

    // Queue up events
    handle.send(IntegrationFsmEvent::Start).await.unwrap();
    handle
        .send(IntegrationFsmEvent::Process("queued".to_string()))
        .await
        .unwrap();

    // Shut down immediately after sending
    handle.shutdown_graceful();

    let final_context = task.await.unwrap();

    // In graceful shutdown, it should process all remaining messages in the queue
    assert_eq!(final_context.transition_count, 2);
    assert_eq!(final_context.job_data, vec!["queued"]);
}
