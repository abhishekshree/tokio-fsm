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
/// # #[derive(Debug, Clone, Copy)]
/// # enum MyFsmState { Running }
/// async fn my_handler() -> Transition<MyFsmState> {
///     // Perform some async logic...
///     Transition::to(MyFsmState::Running)
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

/// Error returned when applying an event to an FSM.
///
/// For direct FSM values, applying an event resolves when the handler has run
/// and the state transition has either succeeded or failed. For spawned FSM
/// handles, applying an event also reports runtime-adapter failures such as a
/// closed event channel or an interrupted in-flight request.
#[derive(Debug, thiserror::Error)]
pub enum ApplyError<E, S> {
    /// The event has no handler for the current state.
    #[error("event is not handled in the current FSM state")]
    Unhandled {
        /// State observed when the event was applied.
        state: S,
        /// Event that was rejected.
        event: E,
    },
    /// The spawned FSM runtime is closed.
    #[error("FSM runtime is closed")]
    Closed(
        /// Event that could not be applied.
        E,
    ),
    /// The FSM stopped before it could answer this apply request.
    #[error("FSM stopped before answering apply request")]
    Interrupted,
    /// The FSM handler failed while processing the event.
    #[error("FSM handler failed while processing event")]
    HandlerFailed,
    /// The handler returned a state that was not declared in its `next` list.
    #[error("FSM handler returned an undeclared transition target")]
    InvalidTransition {
        /// State returned by the handler.
        state: S,
    },
}
