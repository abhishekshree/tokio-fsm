# tokio-fsm

[![Crates.io](https://img.shields.io/crates/v/tokio-fsm.svg)](https://crates.io/crates/tokio-fsm)
[![Docs](https://docs.rs/tokio-fsm/badge.svg)](https://docs.rs/tokio-fsm)
[![CI](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml/badge.svg)](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Compile-time validated async finite state machines for [Tokio](https://tokio.rs).

`tokio-fsm` turns a standard Rust `impl` block into a state machine with generated state/event types, direct event application, an optional Tokio runtime adapter, lifecycle management, and compile-time graph validation. It removes transition boilerplate while keeping handler behavior explicit Rust code.

## Why tokio-fsm?

- **FSM-First API**: Apply events directly to an owned FSM, or use a spawned Tokio adapter when shared handles and state observation are useful.
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
    // Direct ownership has no queue or background task.
    let mut fsm = MyFsm::new(MyContext::default());
    let state = fsm.apply(MyFsmEvent::Start).await.unwrap();
    assert_eq!(state, MyFsmState::Running);
    let context = fsm.into_context();
    assert_eq!(context.count, 1);

    // Spawning returns a Handle and a Task. The Task must be awaited or held.
    let (handle, task) = MyFsm::spawn(MyContext::default());

    // Apply resolves when the FSM processes the event.
    let state = handle.apply(MyFsmEvent::Start).await.unwrap();
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
| `MyFsm` | The FSM value, with `new`, `state`, `context`, `context_mut`, `into_context`, and direct `apply`. |
| `MyFsmState` | An enum of all discovered states. |
| `MyFsmEvent` | An enum of all discovered events and their payloads. |
| `MyFsmHandle` | A cloneable spawned-runtime handle for applying events and querying state. |
| `MyFsmTask` | A `Future` that drives the FSM. Resolves to `Result<Context, TaskError<E>>`. |

## Handler Return Types

Handlers are `async fn` methods that define how the machine moves between states. Most handlers should declare a single `next` state and return:

- `()`: A simple infallible transition to the declared `next` state.
- `Result<(), E>`: A fallible transition to the declared `next` state.
- `Transition<MyFsmState>`: An infallible dynamic transition across states
  listed in `next = [A, B]`.
- `Result<Transition<MyFsmState>, E>`: Dynamic branching across states listed in `next = [A, B]`.

## Lifecycle and Ownership

- **Task Drop**: If you drop the `MyFsmTask` handle, the FSM is aborted immediately. Spawning is marked `#[must_use]` to prevent accidental leaks.
- **Handle Drop**: When the last `MyFsmHandle` is dropped, the internal event channel is closed. The spawned FSM exits after processing any received events.
- **Shutdown**: Call `handle.shutdown()` to cancel the FSM's child token, then `await` the task to retrieve the final context. Cancellation can interrupt an in-flight handler future.
- **Event Application**: `apply` resolves after the FSM processes the event. If the processed state has no handler for that event, it returns `ApplyError::Unhandled`.

## Configuration and Attributes

- `#[fsm(initial = Idle, channel_size = 100)]`: Customize the internal command queue capacity.
- `#[on(state = Idle, event = Start, next = Running)]`: Map an event in a source state to one or more target states.
- `#[fsm(tracing = true)]`: Enable `tracing` instrumentation (requires `tracing` feature).
- `#[fsm(serde = true)]`: Enable `serde` support for states and events (requires `serde` feature).

## Graph Validation & Safety

`tokio-fsm` validates your state machine at compile time to guarantee semantic correctness. If the graph violates these rules, compilation will fail:

1. **Initial State Validity**: The declared `initial` state must exist.
2. **Deterministic Handlers**: Multiple handlers for the exact same `(State, Event)` pair are rejected.
3. **Graph Reachability**: Every declared state must be reachable from the `initial` state.

Refer to the property test refer to the design here at the [validator design](tokio-fsm-macros/src/validation/validator.md)
## Examples

For a full implementation showing Axum integration, multiple FSM instances, and error handling, see the [Axum Order Processing Example](examples/axum_fsm).

## License

MIT
