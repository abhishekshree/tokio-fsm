//! Example: Comparing Manual FSM Implementation vs. tokio-fsm Macro
//!
//! This example shows the exact same FSM logic implemented twice.
//! Compare the amount of boilerplate and manual wiring required for each.

use tokio::sync::{mpsc, watch};
use tokio_fsm::{Transition, fsm};

// --- SHARED DOMAIN LOGIC ---

#[derive(Debug, Clone)]
pub struct Job {
    pub id: u64,
}

#[derive(Debug)]
pub struct Context {
    pub count: usize,
}

// --- OPTION 1: MANUAL IMPLEMENTATION (The "Boilerplate" way) ---

/// Manual state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualState {
    Idle,
    Processing,
}

/// Manual event enum
pub enum ManualEvent {
    Start(Job),
}

/// Manual handle to control the FSM
pub struct ManualHandle {
    pub tx: mpsc::Sender<ManualEvent>,
    pub state_rx: watch::Receiver<ManualState>,
}

/// Manual loop implementation
pub struct ManualFsm {
    state: ManualState,
    context: Context,
}

impl ManualFsm {
    pub fn spawn(context: Context) -> (ManualHandle, tokio::task::JoinHandle<Context>) {
        let (tx, mut rx) = mpsc::channel(100);
        let (state_tx, state_rx) = watch::channel(ManualState::Idle);

        let mut fsm = ManualFsm {
            state: ManualState::Idle,
            context,
        };

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = rx.recv() => {
                        let Some(event) = event else { break };
                        match (fsm.state, event) {
                            (ManualState::Idle, ManualEvent::Start(job)) => {
                                println!("Manual: Starting job {}", job.id);
                                fsm.context.count += 1;
                                fsm.state = ManualState::Processing;
                                let _ = state_tx.send(fsm.state);
                            }
                            _ => {}
                        }
                    }
                }
            }
            fsm.context
        });

        (ManualHandle { tx, state_rx }, handle)
    }
}

// --- OPTION 2: MACRO IMPLEMENTATION (The "tokio-fsm" way) ---

#[fsm(initial = "Idle")]
impl MacroFsm {
    type Context = Context;
    type Error = std::convert::Infallible;

    #[state(Idle)]
    #[event(Start)]
    async fn handle_start(&mut self, job: Job) -> Transition<Processing> {
        println!("Macro: Starting job {}", job.id);
        self.context.count += 1;
        Transition::to(Processing)
    }
}

// --- COMPARISON RUNNER ---

#[tokio::main]
async fn main() {
    println!("=== MANUAL FSM ===");
    let manual_ctx = Context { count: 0 };
    let (manual_handle, _manual_task) = ManualFsm::spawn(manual_ctx);
    manual_handle
        .tx
        .send(ManualEvent::Start(Job { id: 1 }))
        .await
        .unwrap();
    println!("Manual state: {:?}", *manual_handle.state_rx.borrow());

    println!("\n=== MACRO FSM ===");
    let macro_ctx = Context { count: 0 };
    let (macro_handle, _macro_task) = MacroFsm::spawn(macro_ctx);
    macro_handle
        .send(MacroFsmEvent::Start(Job { id: 1 }))
        .await
        .unwrap();
    println!("Macro state: {:?}", macro_handle.current_state());
}
