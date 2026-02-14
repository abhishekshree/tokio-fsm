use criterion::{Criterion, criterion_group, criterion_main};
use tokio::sync::mpsc;

// --- Macro FSM Definition ---
use tokio_fsm::fsm;
use tokio_fsm_core::Transition;

#[derive(Debug, Clone, Default)]
pub struct Context {
    pub counter: usize,
}

#[fsm(initial = "Idle")]
impl MacroFsm {
    type Context = Context;
    type Error = std::convert::Infallible;

    #[event(Ping)]
    async fn on_ping(&mut self) -> Transition<Running> {
        self.context.counter += 1;
        Transition::to(Running)
    }

    #[event(Pong)]
    async fn on_pong(&mut self) -> Transition<Idle> {
        self.context.counter += 1;
        Transition::to(Idle)
    }
}

// --- Manual FSM Definition ---
// Minimal manual implementation for baseline
#[derive(Clone)]
struct ManualFsmHandle {
    tx: mpsc::Sender<ManualEvent>,
    state_rx: tokio::sync::watch::Receiver<ManualState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManualState {
    Idle,
    Running,
}

#[derive(Debug)]
enum ManualEvent {
    Ping,
    Pong,
}

impl ManualFsmHandle {
    fn spawn(mut context: Context) -> (Self, tokio::task::JoinHandle<()>) {
        let (tx, mut rx) = mpsc::channel(100);
        let (state_tx, state_rx) = tokio::sync::watch::channel(ManualState::Idle);

        let handle = tokio::spawn(async move {
            let mut state = ManualState::Idle;

            // Stack pinned sleep optimization (manual)
            let sleep = tokio::time::sleep(tokio::time::Duration::from_secs(3153600000));
            tokio::pin!(sleep);

            loop {
                tokio::select! {
                    _ = &mut sleep => {
                        sleep.as_mut().reset(tokio::time::Instant::now() + tokio::time::Duration::from_secs(3153600000));
                    }
                    Some(event) = rx.recv() => {
                         match (state, event) {
                            (ManualState::Idle, ManualEvent::Ping) => {
                                context.counter += 1;
                                state = ManualState::Running;
                                let _ = state_tx.send(state);
                                sleep.as_mut().reset(tokio::time::Instant::now() + tokio::time::Duration::from_secs(3153600000));
                            }
                            (ManualState::Running, ManualEvent::Pong) => {
                                context.counter += 1;
                                state = ManualState::Idle;
                                let _ = state_tx.send(state);
                                sleep.as_mut().reset(tokio::time::Instant::now() + tokio::time::Duration::from_secs(3153600000));
                            }
                            _ => {}
                         }
                    }
                }
            }
        });

        (Self { tx, state_rx }, handle)
    }
}

// --- Benchmarks ---

fn bench_transitions(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("fsm_transitions");

    // Benchmark Throughput: Send Ping -> Wait for State change -> Send Pong -> Wait for State change
    // This measures round-trip latency including channel overhead and context switching.

    group.bench_function("macro_ping_pong", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let (mut handle, _task) = MacroFsm::spawn(Context::default());
            let start = std::time::Instant::now();
            for _ in 0..iters {
                handle.send(MacroFsmEvent::Ping).await.unwrap();
                handle.wait_for_state(MacroFsmState::Running).await.unwrap();
                handle.send(MacroFsmEvent::Pong).await.unwrap();
                handle.wait_for_state(MacroFsmState::Idle).await.unwrap();
            }
            start.elapsed()
        });
    });

    group.bench_function("manual_ping_pong", |b| {
        b.to_async(&rt).iter_custom(|iters| {
            async move {
                let (handle, _task) = ManualFsmHandle::spawn(Context::default());
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    handle.tx.send(ManualEvent::Ping).await.unwrap();
                    // Manual wait logic
                    let mut rx = handle.state_rx.clone();
                    while *rx.borrow_and_update() != ManualState::Running {
                        rx.changed().await.unwrap();
                    }

                    handle.tx.send(ManualEvent::Pong).await.unwrap();
                    let mut rx = handle.state_rx.clone();
                    while *rx.borrow_and_update() != ManualState::Idle {
                        rx.changed().await.unwrap();
                    }
                }
                start.elapsed()
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_transitions);
criterion_main!(benches);
