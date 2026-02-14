use criterion::{Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;
use tokio_fsm::fsm;

// --- Defined FSM ---
#[derive(Debug, Clone)]
pub struct BenchmarkJob {
    pub id: u64,
}

#[derive(Debug, Default)]
pub struct BenchmarkContext {
    pub count: u64,
}

#[derive(Debug)]
pub enum BenchmarkError {}

#[fsm(initial = "Idle", channel_size = 1024)]
impl BenchmarkFsm {
    type Context = BenchmarkContext;
    type Error = BenchmarkError;

    #[event(Job)]
    async fn handle_job(&mut self, _job: BenchmarkJob) -> tokio_fsm::Transition<Processing> {
        tokio_fsm::Transition::to(Processing)
    }

    #[event(Done)]
    async fn handle_done(&mut self) -> tokio_fsm::Transition<Idle> {
        self.context.count += 1;
        tokio_fsm::Transition::to(Idle)
    }
}

// --- Benchmark Functions ---

fn benchmark_fsm_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("fsm_throughput_1000_cycles", |b| {
        b.to_async(&rt).iter(|| async {
            let (handle, _task) = BenchmarkFsm::spawn(BenchmarkContext::default());

            for i in 0..1000 {
                handle
                    .send(BenchmarkFsmEvent::Job(BenchmarkJob { id: i }))
                    .await
                    .unwrap();
                handle.send(BenchmarkFsmEvent::Done).await.unwrap();
            }

            // Allow cleanup (optional, but good for isolation)
            // handle.shutdown_immediate(); // This might panic if receiver dropped? No.
        })
    });
}

criterion_group!(benches, benchmark_fsm_throughput);
criterion_main!(benches);
