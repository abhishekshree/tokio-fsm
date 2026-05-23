use tokio_fsm::{ApplyError, fsm};

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

    assert_eq!(handle.state(), IntegrationFsmState::Idle);

    // Idle -> Pending
    assert_eq!(
        handle.apply(IntegrationFsmEvent::Start).await.unwrap(),
        IntegrationFsmState::Pending
    );
    handle
        .wait_for_state(IntegrationFsmState::Pending)
        .await
        .unwrap();

    // Pending -> Active (with data)
    handle
        .apply(IntegrationFsmEvent::Process("task1".to_string()))
        .await
        .unwrap();
    handle
        .wait_for_state(IntegrationFsmState::Active)
        .await
        .unwrap();

    // Active -> Done
    assert_eq!(
        handle.apply(IntegrationFsmEvent::Finish).await.unwrap(),
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

    // Apply events before closing the handle.
    handle.apply(IntegrationFsmEvent::Start).await.unwrap();
    handle
        .apply(IntegrationFsmEvent::Process("queued".to_string()))
        .await
        .unwrap();

    // Close the last handle.
    drop(handle);

    let final_context = task.await.unwrap();

    // Once the channel is closed, the FSM drains queued events before exiting.
    assert_eq!(final_context.transition_count, 2);
    assert_eq!(final_context.job_data, vec!["queued"]);
}

#[tokio::test]
async fn test_apply_rejects_unhandled_event() {
    let context = TestContext::default();
    let (handle, task) = IntegrationFsm::spawn(context);

    match handle.apply(IntegrationFsmEvent::Finish).await {
        Err(ApplyError::Unhandled {
            state: IntegrationFsmState::Idle,
            event: IntegrationFsmEvent::Finish,
        }) => {}
        other => panic!("expected apply to reject Finish in Idle, got {other:?}"),
    }

    assert_eq!(handle.state(), IntegrationFsmState::Idle);

    handle.shutdown();
    let final_context = task.await.unwrap();
    assert_eq!(final_context.transition_count, 0);
    assert!(final_context.job_data.is_empty());
}

#[tokio::test]
async fn test_direct_apply_full_lifecycle() {
    let mut fsm = IntegrationFsm::new(TestContext::default());

    assert_eq!(fsm.state(), IntegrationFsmState::Idle);

    assert_eq!(
        fsm.apply(IntegrationFsmEvent::Start).await.unwrap(),
        IntegrationFsmState::Pending
    );
    assert_eq!(fsm.context().transition_count, 1);

    assert_eq!(
        fsm.apply(IntegrationFsmEvent::Process("direct".to_string()))
            .await
            .unwrap(),
        IntegrationFsmState::Active
    );

    let context = fsm.into_context();
    assert_eq!(context.transition_count, 2);
    assert_eq!(context.job_data, vec!["direct"]);
}

#[tokio::test]
async fn test_direct_apply_rejects_unhandled_event() {
    let mut fsm = IntegrationFsm::new(TestContext::default());

    match fsm.apply(IntegrationFsmEvent::Finish).await {
        Err(ApplyError::Unhandled {
            state: IntegrationFsmState::Idle,
            event: IntegrationFsmEvent::Finish,
        }) => {}
        other => panic!("expected apply to reject Finish in Idle, got {other:?}"),
    }

    assert_eq!(fsm.state(), IntegrationFsmState::Idle);
    assert_eq!(fsm.context().transition_count, 0);
}
