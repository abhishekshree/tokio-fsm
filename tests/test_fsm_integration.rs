use tokio_fsm::{SendError, fsm};

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

    #[on(state = Idle, event = Start, next = Pending)]
    async fn handle_start(&mut self) {
        self.context.transition_count += 1;
    }

    #[on(state = Pending, event = Process, next = Active)]
    #[on(state = Active, event = Process, next = Active)]
    async fn handle_process(&mut self, data: String) {
        self.context.transition_count += 1;
        self.context.job_data.push(data);
    }

    #[on(state = Active, event = Finish, next = Done)]
    async fn handle_finish(&mut self) {
        self.context.transition_count += 1;
    }
}

#[tokio::test]
async fn test_fsm_full_lifecycle() {
    let context = TestContext::default();
    let (handle, task) = IntegrationFsm::spawn(context);

    assert_eq!(handle.current_state(), IntegrationFsmState::Idle);

    // Idle -> Pending
    assert_eq!(
        handle.send(IntegrationFsmEvent::Start).await.unwrap(),
        IntegrationFsmState::Pending
    );
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
    assert_eq!(
        handle.send(IntegrationFsmEvent::Finish).await.unwrap(),
        IntegrationFsmState::Done
    );
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

#[tokio::test]
async fn test_fsm_channel_close_shutdown() {
    let context = TestContext::default();
    let (handle, task) = IntegrationFsm::spawn(context);

    // Queue up events
    handle.send(IntegrationFsmEvent::Start).await.unwrap();
    handle
        .send(IntegrationFsmEvent::Process("queued".to_string()))
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
async fn test_send_rejects_unhandled_event() {
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
