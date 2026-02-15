//! # tokio-fsm
//!
//! Compile-time generation of Tokio async finite state machines with explicit
//! Rust behavior and minimal runtime overhead.
//!
//! ## Example
//!
//! ```rust
//! use tokio_fsm::{Transition, fsm};
//!
//! pub struct WorkerContext;
//! pub enum WorkerError {}
//! #[derive(Debug, Clone)]
//! pub struct Job;
//!
//! #[fsm(initial = Idle)]
//! impl WorkerFsm {
//!     type Context = WorkerContext;
//!     type Error = WorkerError;
//!
//!     #[on(state = Idle, event = Job)]
//!     async fn handle_job(&mut self, _job: Job) -> Transition<Working> {
//!         Transition::to(Working)
//!     }
//! }
//! ```

mod core;

#[doc(inline)]
pub use crate::core::*;
#[doc(inline)]
pub use tokio_fsm_macros::*;
