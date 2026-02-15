//! # tokio-fsm
//!
//! **Compile-time validated, zero-overhead async finite state machines for [Tokio](https://tokio.rs).**
//!
//! `tokio-fsm` allows you to define complex asynchronous state machines using a declarative macro.
//! It eliminates the boilerplate of manual event loops, channel management, and state transitions
//! while ensuring that your FSM logic is verified at compile-time.
//!
//! ## Problem Statement
//!
//! Traditional FSM implementations in Rust often fall into two categories:
//! 1. **Hand-rolled**: Extremely fast and flexible, but error-prone. Managing `mpsc` channels,
//!    `tokio::select!` loops, and ensuring "invalid state" transitions don't occur requires significant boilerplate.
//! 2. **Runtime Engines**: Flexible, but often involve virtual dispatch, heavy allocations, or
//!    complex trait hierarchies that add overhead and obfuscate the mental model.
//!
//! `tokio-fsm` provides a **third way**: Write standard Rust `impl` blocks, and let the proc-macro
//! generate the high-performance, type-safe boilerplate for you.
//!
//! ## Core Concepts
//!
//! ### States & Events
//! States and Events are **discovered** automatically from your `#[on]` handlers. You don't need to
//! manually define enums for them—the macro generates a `[FsmName]State` and `[FsmName]Event` enum for you.
//!
//! ### Context
//! The `Context` is the shared, mutable data owned by the FSM. Every handler has access to
//! `&mut self.context`.
//!
//! ### Transitions
//! Handlers return a `Transition<NextState>`. This explicitly defines the next state the FSM
//! should move to. The macro validates that `NextState` is a known state and that the transition
//! is reachable.
//!
//! ### Timeouts
//! `tokio-fsm` supports state-level timeouts via `#[state_timeout]`. These are implemented using
//! single, stack-pinned `tokio::time::Sleep` futures, ensuring zero heap allocations during transitions.
//!
//! ## Architecture
//!
//! 1. **Validation Layer**: At compile-time, the macro builds a directed graph of your FSM. It
//!    verifies that all states are reachable and that transitions are logically consistent.
//! 2. **Codegen Layer**: Generates a tight, state-gated match loop. There is **no runtime engine**;
//!    the generated code is identical to what a senior engineer would write by hand.
//!
//! ## Example: Basic Worker
//!
//! ```rust
//! use tokio_fsm::{fsm, Transition};
//!
//! pub struct WorkerContext { iterations: usize }
//!
//! #[fsm(initial = Idle)]
//! impl WorkerFsm {
//!     type Context = WorkerContext;
//!     type Error = std::convert::Infallible;
//!
//!     #[on(state = Idle, event = Start)]
//!     async fn on_start(&mut self) -> Transition<Running> {
//!         Transition::to(Running)
//!     }
//!
//!     #[on(state = Running, event = Tick)]
//!     async fn on_tick(&mut self) -> Transition<Running> {
//!         self.context.iterations += 1;
//!         Transition::to(Running)
//!     }
//! }
//! ```
//!
//! ## FAQ & Troubleshooting
//!
//! ### Why is my event ignored?
//! If an event is sent while the FSM is in a state where no `#[on(state = current, event = event)]`
//! handler is defined, the event is silently dropped by default. This is intentional to prevent
//! deadlocks in high-throughput systems.
//!
//! ### Compile Error: "State X not found"
//! Ensure that state `X` is either the `initial` state or is used as a target in a `Transition<X>`
//! or a source in an `#[on(state = X, ...)]` attribute.
//!
//! ### Compile Error: "Event Y not found"
//! Ensure that event `Y` is defined in at least one `#[on(..., event = Y)]` attribute.

mod core;

#[doc(inline)]
pub use tokio_fsm_macros::*;

#[doc(inline)]
pub use crate::core::*;
