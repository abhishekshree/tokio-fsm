# tokio-fsm

[![Crates.io](https://img.shields.io/crates/v/tokio-fsm.svg)](https://crates.io/crates/tokio-fsm)
[![Docs](https://docs.rs/tokio-fsm/badge.svg)](https://docs.rs/tokio-fsm)
[![CI](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml/badge.svg)](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Compile-time validated, actor-style async finite state machines for [Tokio](https://tokio.rs).

`tokio-fsm` turns a standard Rust `impl` block into a Tokio-driven state machine with generated state/event types, a handle API, lifecycle management, and compile-time graph validation. It removes the boilerplate of manual event loops and channel management while keeping handler behavior explicit Rust code.

## Why tokio-fsm?

- **Actor-Style Runtime**: Generates a Tokio task backed by bounded `mpsc` events, `watch` state observation, cancellation, and a direct `match`-based dispatcher.
- **Async First**: All handlers are native `async fn` methods.
- **Compile-Time Safety**: Validates state reachability and transition contracts during compilation using `petgraph`.
- **Deterministic Lifecycle**: Explicit ownership model via a `Task` handle that ensures resources are cleaned up if the caller drops the FSM.

## Quick Start

```rust
use tokio_fsm::fsm;

#[derive(Debug, Default)]
pub struct MyContext {
    count: usize,
}

#[fsm(initial = Idle)]
impl MyFsm {
    type Context = MyContext;
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start, next = Running)]
    async fn handle_start(&mut self) {
        self.context.count += 1;
    }

    #[on(state = Running, event = Stop, next = Idle)]
    async fn handle_stop(&mut self) {
    }
}

#[tokio::main]
async fn main() {
    // Spawning returns a Handle and a Task. The Task must be awaited or held.
    let (handle, task) = MyFsm::spawn(MyContext::default());

    // Send resolves when the FSM processes the event.
    let state = handle.send(MyFsmEvent::Start).await.unwrap();
    assert_eq!(state, MyFsmState::Running);
    
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

Handlers are `async fn` methods that define how the machine moves between states. Most handlers should declare a single `next` state and return:

- `()`: A simple infallible transition to the declared `next` state.
- `Result<(), E>`: A fallible transition to the declared `next` state.
- `Result<Transition<MyFsmState>, E>`: Dynamic branching across states listed in `next = [A, B]`.

## Lifecycle and Ownership

- **Task Drop**: If you drop the `MyFsmTask` handle, the FSM is aborted immediately. Spawning is marked `#[must_use]` to prevent accidental leaks.
- **Handle Drop**: When the last `MyFsmHandle` is dropped, the internal event channel is closed. The FSM will exit after processing any remaining queued events.
- **Shutdown**: Call `handle.shutdown()` to cancel the FSM's child token, then `await` the task to retrieve the final context. Cancellation can interrupt an in-flight handler future.
- **Ordered Sends**: `send` queues one event and resolves after the FSM processes it. If the processed state has no handler for that event, `send` returns `SendError::Unhandled`.

## Configuration and Attributes

- `#[fsm(initial = Idle, channel_size = 100)]`: Customize the internal command queue capacity.
- `#[on(state = Idle, event = Start, next = Running)]`: Map an event in a source state to one or more target states.
- `#[fsm(tracing = true)]`: Enable `tracing` instrumentation (requires `tracing` feature).
- `#[fsm(serde = true)]`: Enable `serde` support for states and events (requires `serde` feature).

## Graph Validation & Safety

`tokio-fsm` validates reachability and transition contracts at compile time, so declared states and event handlers stay aligned as the workflow evolves.

## Examples

For a full implementation showing Axum integration, multiple FSM instances, and error handling, see the [Axum Order Processing Example](examples/axum_fsm).

## License

MIT
