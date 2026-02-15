//! Core runtime types for tokio-fsm.

/// Represents a state transition in the FSM.
///
/// This type is returned by FSM handlers to indicate which state the machine
/// should transition to next. It is usually created via the [`Transition::to`]
/// helper.
///
/// # Example
///
/// ```rust
/// # use tokio_fsm::Transition;
/// # struct Running;
/// async fn my_handler() -> Transition<Running> {
///     // Perform some async logic...
///     Transition::to(Running)
/// }
/// ```
#[derive(Debug)]
pub enum Transition<T> {
    /// Transition to the specified target state.
    To(T),
}

impl<T> Transition<T> {
    /// Creates a new transition to the specified target state.
    ///
    /// The target state must be a valid state defined within the FSM.
    #[must_use]
    pub fn to(state: T) -> Self {
        Self::To(state)
    }

    /// Extracts the target state from the transition.
    ///
    /// Internal-only: This is typically used by the generated event loop.
    #[must_use]
    pub fn into_state(self) -> T {
        match self {
            Self::To(state) => state,
        }
    }
}

/// Shutdown mode for the FSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownMode {
    /// Graceful shutdown: The event loop continues to process all remaining
    /// events currently in the queue before terminating and returning the
    /// context.
    Graceful,
    /// Immediate shutdown: The event loop terminates immediately, dropping any
    /// unprocessed events in the queue, and returns the current context.
    Immediate,
}

/// Error type returned by the FSM background task.
///
/// This enum distinguishes between logical errors returned by your FSM handlers
/// and runtime failures of the Tokio task itself (e.g., panics or
/// cancellation).
///
/// # Type Parameters
///
/// * `E`: The logical error type defined in your `impl` block via `type Error =
///   ...;`.
#[derive(Debug, thiserror::Error)]
pub enum TaskError<E> {
    /// The FSM handler returned a logical error.
    #[error("FSM error: {0}")]
    Fsm(E),
    /// The background task failed due to a panic or external cancellation.
    #[error("Task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}
