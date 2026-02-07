//! # tokio-fsm
//!
//! Compile-time generation of Tokio async finite state machines with explicit Rust behavior
//! and zero runtime overhead.
//!
//! ## Example
//!
//! ```rust
//! use tokio_fsm::{fsm, Transition};
//!
//! pub struct WorkerContext;
//! pub enum WorkerError {}
//! #[derive(Debug, Clone)]
//! pub struct Job;
//!
//! #[fsm(initial = "Idle")]
//! impl WorkerFsm {
//!     type Context = WorkerContext;
//!     type Error = WorkerError;
//!
//!     #[event(Job)]
//!     async fn handle_job(&mut self, _job: Job) -> Transition<Working> {
//!         Transition::to(Working)
//!     }
//! }
//! ```

pub use tokio_fsm_core::{ShutdownMode, Transition, parse_duration};
pub use tokio_fsm_macros::fsm;
