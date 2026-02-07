//! # tokio-fsm
//!
//! Compile-time generation of Tokio async finite state machines with explicit Rust behavior
//! and zero runtime overhead.
//!
//! ## Example
//!
//! ```rust,no_run
//! use tokio_fsm::{fsm, Transition};
//!
//! #[fsm(initial = "Idle", channel_size = 100)]
//! impl WorkerFsm {
//!     type Context = WorkerContext;
//!     type Error = WorkerError;
//!     
//!     #[event(Job)]
//!     async fn handle_job(&mut self, job: Job) -> Result<Transition<Working>, Transition<Failed>> {
//!         // Your async logic here
//!         Ok(Transition::to(Working))
//!     }
//! }
//! ```

pub use tokio_fsm_core::{parse_duration, ShutdownMode, Transition};
pub use tokio_fsm_macros::fsm;

