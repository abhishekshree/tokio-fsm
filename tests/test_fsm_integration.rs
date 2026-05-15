use std::time::Duration;

use tokio_fsm::{SendError, Transition, fsm};

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
    handle.shutdown();
    let final_context = task.await.unwrap();

    assert_eq!(final_context.transition_count, 3);
    assert_eq!(final_context.job_data, vec!["task1"]);
}

#[tokio::test(start_paused = true)]
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
    tokio::time::advance(Duration::from_millis(200)).await;
    handle
        .wait_for_state(IntegrationFsmState::Failed)
        .await
        .unwrap();

    handle.shutdown();
    let final_context = task.await.unwrap();
    assert_eq!(final_context.transition_count, 2); // Start + Timeout
}

#[tokio::test]
async fn test_fsm_channel_close_shutdown() {
    let context = TestContext::default();
    let (handle, task) = IntegrationFsm::spawn(context);

    // Queue up events
    handle.enqueue(IntegrationFsmEvent::Start).await.unwrap();
    handle
        .enqueue(IntegrationFsmEvent::Process("queued".to_string()))
        .await
        .unwrap();

    // Close the last sender by dropping the handle.
    drop(handle);

    let final_context = task.await.unwrap();

    // Once the channel is closed, the FSM drains queued events before exiting.
    assert_eq!(final_context.transition_count, 2);
    assert_eq!(final_context.job_data, vec!["queued"]);
}

#[tokio::test]
async fn test_send_rejects_unhandled_event_without_enqueueing() {
    let context = TestContext::default();
    let (handle, task) = IntegrationFsm::spawn(context);

    match handle.send(IntegrationFsmEvent::Finish).await {
        Err(SendError::Unhandled {
            state: IntegrationFsmState::Idle,
            event: IntegrationFsmEvent::Finish,
        }) => {}
        other => panic!("expected send to reject Finish in Idle, got {other:?}"),
    }

    assert_eq!(handle.current_state(), IntegrationFsmState::Idle);

    handle.shutdown();
    let final_context = task.await.unwrap();
    assert_eq!(final_context.transition_count, 0);
    assert!(final_context.job_data.is_empty());
}

#[tokio::test]
async fn test_enqueue_preserves_raw_queue_semantics() {
    let context = TestContext::default();
    let (handle, task) = IntegrationFsm::spawn(context);

    handle.enqueue(IntegrationFsmEvent::Finish).await.unwrap();
    assert_eq!(handle.current_state(), IntegrationFsmState::Idle);

    handle.shutdown();
    let final_context = task.await.unwrap();
    assert_eq!(final_context.transition_count, 0);
    assert!(final_context.job_data.is_empty());
}
