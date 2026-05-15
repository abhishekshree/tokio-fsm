//! Core runtime types for tokio-fsm.

/// Represents a state transition in the FSM.
///
/// This type is returned by FSM handlers to indicate which state the machine
/// should transition to next. It is usually created via the [`Transition::to`]
/// helper.
///
/// # Examples
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

/// Error type returned by the FSM background task.
///
/// This enum distinguishes between logical errors returned by your FSM handlers
/// and runtime failures of the Tokio task itself (for example, panics or task
/// aborts).
///
/// # Type Parameters
///
/// * `E`: The logical error type defined in your `impl` block via `type Error =
///   ...;`.
///
/// # Examples
///
/// ```rust,ignore
/// use tokio_fsm::TaskError;
///
/// // Example of a match against a task's result
/// match task.await {
///     Ok(final_context) => println!("FSM finished normally."),
///     Err(TaskError::Fsm(e)) => println!("FSM aborted with a logical error: {}", e),
///     Err(TaskError::Join(e)) => println!("Tokio task failed (e.g. panicked): {}", e),
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum TaskError<E> {
    /// The FSM handler returned a logical error, triggering a shutdown.
    ///
    /// This variant is used when your FSM handler returns `Result::Err(...)`.
    #[error("FSM error: {0}")]
    Fsm(E),
    /// The background task failed due to a panic or explicit task abort.
    ///
    /// This wraps a [`tokio::task::JoinError`].
    #[error("Task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// Error returned by generated `send` handle methods.
///
/// Sending validates the event against the FSM's current observed state before
/// enqueueing it. This catches events that cannot be handled in the current
/// state without relying on the background task to drop them later.
#[derive(Debug, thiserror::Error)]
pub enum SendError<E, S> {
    /// The event has no handler for the current state.
    #[error("event is not handled in the current FSM state")]
    Unhandled {
        /// State observed when the send was attempted.
        state: S,
        /// Event that was rejected.
        event: E,
    },
    /// The FSM event queue is closed.
    #[error("FSM event queue is closed")]
    Closed(
        /// Event that could not be enqueued.
        E,
    ),
}
