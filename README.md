# tokio-fsm

[![Crates.io](https://img.shields.io/crates/v/tokio-fsm.svg)](https://crates.io/crates/tokio-fsm)
[![Docs](https://docs.rs/tokio-fsm/badge.svg)](https://docs.rs/tokio-fsm)
[![CI](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml/badge.svg)](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)


Compile-time validated Tokio async finite state machines with explicit Rust behavior and minimal runtime overhead.

`tokio-fsm` allows you to define complex asynchronous state machines using a declarative macro. It handles the boilerplate of event loops, channel management, and state transitions, allowing you to focus on your business logic.

## Features

- **Declarative FSMs**: Define states and events using standard Rust `impl` blocks.
- **State-Gated Events**: Handlers declare which states they are valid in via `#[state(...)]`.
- **Async First**: All handlers are `async`, designed to work seamlessly with the Tokio runtime.
- **Compile-time Validation**: Uses `petgraph` to verify state reachability and valid transitions at compile-time.
- **State Timeouts**: Easily configure timeouts for specific states that trigger auto-transitions.
- **Type-Safe Transitions**: Ensures you only transition to valid states defined in your machine.

## Quick Start

```rust
use tokio_fsm::{fsm, Transition};

#[derive(Debug, Default)]
pub struct MyContext { count: usize }

#[fsm(initial = "Idle")]
impl MyFsm {
    type Context = MyContext;
    type Error = std::convert::Infallible;

    #[state(Idle)]
    #[event(Start)]
    async fn handle_start(&mut self) -> Transition<Running> {
        self.context.count += 1;
        Transition::to(Running)
    }

    #[state(Running)]
    #[event(Stop)]
    async fn handle_stop(&mut self) -> Transition<Idle> {
        Transition::to(Idle)
    }
}

#[tokio::main]
async fn main() {
    let (handle, task) = MyFsm::spawn(MyContext::default());
    
    handle.send(MyFsmEvent::Start).await.unwrap();
    println!("Current state: {:?}", handle.current_state());
    
    handle.shutdown_graceful();
    let final_context = task.await.unwrap();
    println!("Total transitions: {}", final_context.count);
}
```

## Comparisons

See [tokio-fsm/examples/comparison.rs](tokio-fsm/examples/comparison.rs) for a side-by-side comparison of a manual implementation vs. the `tokio-fsm` way.

## Documentation

Detailed documentation on attributes:

- `#[fsm(initial = "...", channel_size = 100)]`: Entry point for the FSM.
- `#[state(Idle, Running)]`: Declares which states a handler is valid in (required on event handlers).
- `#[event(EventName)]`: Maps a method to a specific event.
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
