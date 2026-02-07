//! Core runtime types for tokio-fsm.

use std::time::Duration;

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

/// Internal utility to parse durations using `humantime`.
///
/// This is public for use by the proc macro, but not part of the stable API.
#[doc(hidden)]
pub fn parse_duration(s: &str) -> Result<Duration, humantime::DurationError> {
    s.parse::<humantime::Duration>().map(|d| d.into())
}
