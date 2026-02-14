//! Core runtime types for tokio-fsm.

/// Represents a state transition in the FSM.
#[derive(Debug)]
pub enum Transition<T> {
    /// Transition to a new state.
    To(T),
}

impl<T> Transition<T> {
    /// Create a simple transition to a new state.
    pub fn to(state: T) -> Self {
        Self::To(state)
    }

    /// Extract the target state.
    pub fn into_state(self) -> T {
        match self {
            Self::To(state) => state,
        }
    }
}

/// Shutdown mode for graceful or immediate termination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownMode {
    /// Graceful shutdown: process remaining events in the queue before
    /// terminating.
    Graceful,
    /// Immediate shutdown: terminate immediately without processing remaining
    /// events.
    Immediate,
}

/// Error type returned by the FSM task.
#[derive(Debug, thiserror::Error)]
pub enum TaskError<E> {
    /// The FSM terminated with a logical error.
    #[error("FSM error: {0}")]
    Fsm(E),
    /// The FSM task panicked or was cancelled.
    #[error("Task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}
