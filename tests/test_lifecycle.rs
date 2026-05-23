use tokio::time::{Duration, timeout};
use tokio_fsm::{ApplyError, fsm};
use tokio_util::sync::CancellationToken;

#[fsm(initial = Idle)]
impl LifecycleFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Tick, next = Running)]
    async fn on_tick(&mut self) {}
}

#[tokio::test]
async fn test_fsm_abort_on_drop() {
    let (handle, task) = LifecycleFsm::spawn(());

    // Apply an event to ensure it's running.
    handle.apply(LifecycleFsmEvent::Tick).await.unwrap();
    handle
        .wait_for_state(LifecycleFsmState::Running)
        .await
        .unwrap();

    // Drop the task handle - this should abort the FSM
    drop(task);

    // Poll until applying fails to avoid scheduler-sensitive fixed sleeps.
    timeout(Duration::from_millis(200), async {
        loop {
            if handle.apply(LifecycleFsmEvent::Tick).await.is_err() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("expected apply to fail shortly after task drop");
}

#[tokio::test]
async fn test_fsm_manual_shutdown() {
    let (handle, task) = LifecycleFsm::spawn(());

    handle.shutdown();

    let res = task.await;
    assert!(res.is_ok(), "Task should return Ok on graceful shutdown");

    assert!(
        matches!(
            handle.apply(LifecycleFsmEvent::Tick).await,
            Err(ApplyError::Closed(_))
        ),
        "Expected apply to fail after shutdown"
    );
}

#[tokio::test]
async fn test_shutdown_does_not_cancel_parent_token() {
    let parent = CancellationToken::new();
    let (handle, task) = LifecycleFsm::spawn_with_token((), parent.clone());

    handle.shutdown();

    assert!(
        !parent.is_cancelled(),
        "handle.shutdown() must not cancel the caller's token"
    );
    assert!(
        task.await.is_ok(),
        "task should stop cleanly when the child token is cancelled"
    );
}

#[tokio::test]
async fn test_parent_token_cancels_fsm() {
    let parent = CancellationToken::new();
    let (_handle, task) = LifecycleFsm::spawn_with_token((), parent.clone());

    parent.cancel();

    assert!(
        task.await.is_ok(),
        "parent token cancellation should propagate to the FSM"
    );
}

pub struct BlockingContext {
    started_tx: Option<tokio::sync::oneshot::Sender<()>>,
    completed: bool,
}

#[fsm(initial = BlockingIdle)]
impl BlockingFsm {
    type Context = BlockingContext;
    type Error = std::convert::Infallible;

    #[on(state = BlockingIdle, event = Run, next = BlockingDone)]
    async fn on_run(&mut self) {
        if let Some(started_tx) = self.context.started_tx.take() {
            let _ = started_tx.send(());
        }

        tokio::time::sleep(Duration::from_secs(60)).await;
        self.context.completed = true;
    }
}

#[tokio::test]
async fn test_shutdown_cancels_long_running_handler_promptly() {
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let context = BlockingContext {
        started_tx: Some(started_tx),
        completed: false,
    };
    let (handle, task) = BlockingFsm::spawn(context);

    let apply = tokio::spawn({
        let handle = handle.clone();
        async move { handle.apply(BlockingFsmEvent::Run).await }
    });
    started_rx.await.unwrap();

    handle.shutdown();

    assert!(matches!(apply.await.unwrap(), Err(ApplyError::Interrupted)));
    let context = tokio::time::timeout(Duration::from_millis(100), task)
        .await
        .expect("shutdown should interrupt a blocked handler")
        .unwrap();
    assert!(
        !context.completed,
        "cancelled handler must not run to completion"
    );
}

#[cfg(feature = "tracing")]
#[fsm(initial = TracedIdle, tracing = true)]
impl TracedLifecycleFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = TracedIdle, event = Tick, next = TracedDone)]
    async fn on_tick(&mut self) {}
}

#[cfg(feature = "tracing")]
#[tokio::test]
async fn test_tracing_enabled_fsm_runs() {
    let (handle, task) = TracedLifecycleFsm::spawn(());

    handle.apply(TracedLifecycleFsmEvent::Tick).await.unwrap();
    handle
        .wait_for_state(TracedLifecycleFsmState::TracedDone)
        .await
        .unwrap();

    handle.shutdown();
    assert!(task.await.is_ok());
}
