use criterion::{Criterion, criterion_group, criterion_main};
use tokio::sync::mpsc;
use tokio_fsm::fsm;
use tokio_fsm_core::Transition;

#[derive(Debug, Clone, Default)]
pub struct Context {
    pub counter: usize,
}

#[fsm(initial = Idle)]
impl MacroFsm {
    type Context = Context;
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Ping)]
    async fn on_ping(&mut self) -> Transition<Running> {
        self.context.counter += 1;
        Transition::to(Running)
    }

    #[on(state = Running, event = Pong)]
    async fn on_pong(&mut self) -> Transition<Idle> {
        self.context.counter += 1;
        Transition::to(Idle)
    }
}

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

struct ManualFsm {
    context: Context,
}

impl ManualFsm {
    async fn on_ping(&mut self) -> Transition<ManualState> {
        self.context.counter = self.context.counter.wrapping_add(1);
        Transition::to(ManualState::Running)
    }

    async fn on_pong(&mut self) -> Transition<ManualState> {
        self.context.counter = self.context.counter.wrapping_add(1);
        Transition::to(ManualState::Idle)
    }
}

impl ManualFsmHandle {
    fn spawn(context: Context) -> (Self, tokio::task::JoinHandle<()>) {
        let (tx, mut rx) = mpsc::channel(100);
        let (state_tx, state_rx) = tokio::sync::watch::channel(ManualState::Idle);
        let (shutdown_tx, mut shutdown_rx) =
            tokio::sync::watch::channel(None::<tokio_fsm_core::ShutdownMode>);

        let handle = tokio::spawn(async move {
            let mut fsm = ManualFsm { context };
            let mut state = ManualState::Idle;
            let _shutdown_tx = shutdown_tx;

            let sleep = tokio::time::sleep(tokio::time::Duration::from_secs(3153600000));
            tokio::pin!(sleep);

            loop {
                tokio::select! {
                    _ = &mut sleep => {
                        sleep.as_mut().reset(tokio::time::Instant::now() + tokio::time::Duration::from_secs(3153600000));
                    }
                    _ = shutdown_rx.changed() => {
                        break;
                    }
                    res = rx.recv() => {
                        match res {
                            Some(event) => {
                                match (state, event) {
                                    (ManualState::Idle, ManualEvent::Ping) => {
                                        let transition = fsm.on_ping().await;
                                        state = transition.into_state();
                                        let _ = state_tx.send(state);
                                        sleep.as_mut().reset(tokio::time::Instant::now() + tokio::time::Duration::from_secs(3153600000));
                                    }
                                    (ManualState::Running, ManualEvent::Pong) => {
                                        let transition = fsm.on_pong().await;
                                        state = transition.into_state();
                                        let _ = state_tx.send(state);
                                        sleep.as_mut().reset(tokio::time::Instant::now() + tokio::time::Duration::from_secs(3153600000));
                                    }
                                    _ => {}
                                }
                            }
                            None => break,
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

    group.bench_function("latency_macro_ping_pong", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let (handle, _task) = MacroFsm::spawn(Context::default());
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

    group.bench_function("latency_manual_ping_pong", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let (handle, _task) = ManualFsmHandle::spawn(Context::default());
            let start = std::time::Instant::now();
            for _ in 0..iters {
                handle.tx.send(ManualEvent::Ping).await.unwrap();
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
        });
    });

    group.finish();

    let mut group = c.benchmark_group("fsm_throughput");
    group.throughput(criterion::Throughput::Elements(1));

    group.bench_function("throughput_macro_fire", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let (handle, _task) = MacroFsm::spawn(Context::default());
            let start = std::time::Instant::now();
            for _ in 0..iters {
                handle.send(MacroFsmEvent::Ping).await.unwrap();
            }
            start.elapsed()
        });
    });

    group.bench_function("throughput_manual_fire", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let (handle, _task) = ManualFsmHandle::spawn(Context::default());
            let start = std::time::Instant::now();
            for _ in 0..iters {
                handle.tx.send(ManualEvent::Ping).await.unwrap();
            }
            start.elapsed()
        });
    });
    group.finish();
}

criterion_group!(benches, bench_transitions);
criterion_main!(benches);
