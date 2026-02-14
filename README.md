# tokio-fsm

[![Crates.io](https://img.shields.io/crates/v/tokio-fsm.svg)](https://crates.io/crates/tokio-fsm)
[![Docs](https://docs.rs/tokio-fsm/badge.svg)](https://docs.rs/tokio-fsm)
[![CI](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml/badge.svg)](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)


Compile-time validated Tokio async finite state machines with explicit Rust behavior and minimal runtime overhead.

`tokio-fsm` allows you to define complex asynchronous state machines using a declarative macro. It handles the boilerplate of event loops, channel management, and state transitions.

## Features

- **Declarative FSMs**: Define states and events using standard Rust `impl` blocks.
- **Unified Handlers**: Use `#[on(state = X, event = Y)]` to map states and events to code.
- **Async First**: All handlers are `async`.
- **Compile-time Validation**: Verifies state reachability and valid transitions at compile-time.
- **Type-Safe Transitions**: Ensures you only transition to valid states defined in your machine.

## Defining States and Events

You don't need to manually define enums or structs for your states and events. The `#[fsm]` macro **discovers** them from your implementation:

- **States**: Are discovered from the `initial` parameter, the `state` field in `#[on]`, and the `Transition<State>` return types.
- **Events**: Are discovered from the `event` field in `#[on]`.
- **Event Data**: If a handler has a second argument (e.g., `fn handle(&mut self, data: MyData)`), the event will carry `MyData` as its payload.

## Quick Start

```rust
use tokio_fsm::{fsm, Transition};

#[derive(Debug, Default)]
pub struct MyContext { count: usize }

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
    let (handle, task) = MyFsm::spawn(MyContext::default());
    
    // Events are generated as an enum: [FsmName]Event
    handle.send(MyFsmEvent::Start).await.unwrap();
    
    handle.shutdown_graceful();
    let final_context = task.await.unwrap();
}
```

## Documentation

- `#[fsm(initial = Idle, channel_size = 100)]`: Entry point for the FSM. `initial` takes the state name directly.
- `#[on(state = Idle, event = Start)]`: Maps a handler to a specific state and event. You can have multiple `#[on]` attributes on one method for multi-state handlers.
- `#[state_timeout(duration = "30s")]`: Configures a timeout for the state reached after this transition.
- `#[on_timeout]`: Specifies the handler that executes when a state times out.

## Architecture & Correctness

`tokio-fsm` employs a 2-layer architecture:

1.  **Validation Layer**: Parses the `impl` block, extracts semantic structure, and validates the FSM graph using `petgraph` at compile-time.
2.  **Codegen Layer**: Generates strictly typed Rust code with state-gated event matching.

### Optimizations
- **Stack-Pinned Timeouts**: State timeouts use a single, reused `tokio::time::Sleep` future pinned to the stack, avoiding `Box::pin` allocations on every transition.
- **Bounded Channels**: Events are processed via a bounded `mpsc` channel to apply backpressure.

### Error Handling
The background `Task` returns `Result<Context, TaskError<E>>`, where `TaskError` explicitly distinguishes between FSM logical errors and runtime task failures (panics/cancellation).

## License

MIT
