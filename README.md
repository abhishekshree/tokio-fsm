# tokio-fsm

[![Crates.io](https://img.shields.io/crates/v/tokio-fsm.svg)](https://crates.io/crates/tokio-fsm)
[![Docs](https://docs.rs/tokio-fsm/badge.svg)](https://docs.rs/tokio-fsm)
[![CI](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml/badge.svg)](https://github.com/abhishekshree/tokio-fsm/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)


Compile-time generation of Tokio async finite state machines with explicit Rust behavior and zero runtime overhead.

`tokio-fsm` allows you to define complex asynchronous state machines using a declarative macro. It handles the boilerplate of event loops, channel management, and state transitions, allowing you to focus on your business logic.

## Features

- **Declarative FSMs**: Define states and events using standard Rust `impl` blocks.
- **Async First**: All handlers are `async`, designed to work seamlessly with the Tokio runtime.
- **Zero Runtime Overhead**: The macro generates idiomatic Rust code for the event loop—no trait objects or dynamic dispatch.
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

    #[event(Start)]
    async fn handle_start(&mut self) -> Transition<Running> {
        self.context.count += 1;
        Transition::to(Running)
    }

    #[event(Stop)]
    async fn handle_stop(&mut self) -> Transition<Idle> {
        Transition::to(Idle)
    }
}

#[tokio::main]
async fn main() {
    let (handle, task) = MyFsm::spawn(MyContext::default());
    
    handle.send(Event::Start).await.unwrap();
    println!("Current state: {:?}", handle.current_state());
    
    handle.shutdown_graceful();
    let final_context = task.await.unwrap();
    println!("Total transitions: {}", final_context.count);
}
```

## Comparisons

See [tokio-fsm/examples/comparison.rs](tokio-fsm/examples/comparison.rs) for a side-by-side comparison of a manual implementation vs. the `tokio-fsm` way. The macro reduces boilerplate by ~70% and ensures safety through graph validation.

## Documentation

Detailed documentation on attributes:

- `#[fsm(initial = "...", channel_size = 100)]`: Entry point for the FSM.
- `#[event(EventName)]`: Maps a method to a specific event.
- `#[state_timeout(duration = "30s")]`: Configures a timeout for the state reached after this transition.
- `#[on_timeout]`: Specifies the handler that executes when a state times out.

## License

MIT
