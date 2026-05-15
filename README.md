# tokio-fsm

[![Crates.io](https://img.shields.io/crates/v/tokio-fsm.svg)](https://crates.io/crates/tokio-fsm)
[![Docs](https://docs.rs/tokio-fsm/badge.svg)](https://docs.rs/tokio-fsm)
[![CI](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml/badge.svg)](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Compile-time validated, actor-style async finite state machines for [Tokio](https://tokio.rs).

`tokio-fsm` turns a standard Rust `impl` block into a Tokio-driven state machine with generated state/event types, a handle API, lifecycle management, and compile-time graph validation. It removes the boilerplate of manual event loops, channel management, and timeout wiring while keeping handler behavior explicit Rust code.

## Why tokio-fsm?

- **Actor-Style Runtime**: Generates a Tokio task backed by bounded `mpsc` events, `watch` state observation, cancellation, and a direct `match`-based dispatcher.
- **Async First**: All handlers are native `async fn` methods.
- **Compile-Time Safety**: Validates state reachability and transition contracts during compilation using `petgraph`.
- **Deterministic Lifecycle**: Explicit ownership model via a `Task` handle that ensures resources are cleaned up if the caller drops the FSM.

## Quick Start

```rust
use tokio_fsm::{fsm, Transition};

#[derive(Debug, Default)]
pub struct MyContext {
    count: usize,
}

#[fsm(initial = Idle)]
impl MyFsm {
    type Context = MyContext;
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start)]
    async fn handle_start(&mut self) -> Transition<Running> {
        self.context.count += 1;
        Transition::to(Running)
    }

    #[on(state = Running, event = Stop)]
    async fn handle_stop(&mut self) -> Transition<Idle> {
        Transition::to(Idle)
    }
}

#[tokio::main]
async fn main() {
    // Spawning returns a Handle and a Task. The Task must be awaited or held.
    let (handle, task) = MyFsm::spawn(MyContext::default());

    // Send validates that the current observed state can handle the event.
    handle.send(MyFsmEvent::Start).await.unwrap();
    
    // Observer state changes
    handle.wait_for_state(MyFsmState::Running).await.unwrap();

    // Cooperative shutdown
    handle.shutdown();
    let final_context = task.await.unwrap();
    assert_eq!(final_context.count, 1);
}
```

## Generated API

For an `impl` named `MyFsm`, the macro generates:

| Type | Description |
|------|-------------|
| `MyFsmState` | An enum of all discovered states. |
| `MyFsmEvent` | An enum of all discovered events and their payloads. |
| `MyFsmHandle` | A cloneable handle for sending events and querying state. |
| `MyFsmTask` | A `Future` that drives the FSM. Resolves to `Result<Context, TaskError<E>>`. |

## Handler Return Types

Handlers are `async fn` methods that define how the machine moves between states. They can return:

- [`Transition<Next>`](crate::Transition): A simple transition to a target state.
- `Result<Transition<Next>, Transition<Other>>`: Branching logic where both paths lead to valid states.
- `Result<Transition<Next>, E>`: A fallible handler where an error will terminate the FSM and return [`TaskError::Fsm(E)`](crate::TaskError::Fsm).

## Lifecycle and Ownership

- **Task Drop**: If you drop the `MyFsmTask` handle, the FSM is aborted immediately. Spawning is marked `#[must_use]` to prevent accidental leaks.
- **Handle Drop**: When the last `MyFsmHandle` is dropped, the internal event channel is closed. The FSM will exit after processing any remaining queued events.
- **Shutdown**: Call `handle.shutdown()` to cancel the FSM's child token, then `await` the task to retrieve the final context. Cancellation can interrupt an in-flight handler future.
- **Checked Events**: `send` validates the event against the current observed state before enqueueing. Use `enqueue` / `try_enqueue` only when you intentionally want raw queue semantics; an event with no handler in the eventual runtime state is dropped by the dispatcher.

## Configuration and Attributes

- `#[fsm(initial = Idle, channel_size = 100)]`: Customize the internal `mpsc` capacity.
- `#[state_timeout(duration = "30s")]`: Rearm a single pinned timer whenever this state is reached.
- `#[on_timeout]`: Define the handler for state timeouts.
- `#[fsm(tracing = true)]`: Enable `tracing` instrumentation (requires `tracing` feature).
- `#[fsm(serde = true)]`: Enable `serde` support for states and events (requires `serde` feature).

## Graph Validation & Safety

`tokio-fsm` goes beyond simple type-checking. It validates your state machine as a formal mathematical graph $(Q, \Sigma, \delta, q_0, F)$. 

For more information and a live comparison showing why `tokio-fsm` catches errors that typestate might miss, see the [Validation Comparison Example](examples/validation_comparison).

## Examples

For a full implementation showing Axum integration, multiple FSM instances, and error handling, see the [Axum Order Processing Example](examples/axum_fsm).

## License

MIT
