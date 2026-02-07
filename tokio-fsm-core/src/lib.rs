//! Core runtime types for tokio-fsm.

use std::time::Duration;

/// Represents a state transition in the FSM.
#[derive(Debug)]
pub enum Transition<T> {
    /// Transition to a new state.
    To(T),
    /// Transition to a new state with associated data.
    ToWithData(T, Box<dyn std::error::Error + Send + Sync>),
}

impl<T> Transition<T> {
    /// Create a simple transition to a new state.
    pub fn to(state: T) -> Self {
        Self::To(state)
    }

    /// Create a transition to a new state with associated error data.
    pub fn to_with_data<E>(state: T, error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::ToWithData(state, Box::new(error))
    }

    /// Extract the target state, discarding any error data.
    pub fn into_state(self) -> T {
        match self {
            Self::To(state) | Self::ToWithData(state, _) => state,
        }
    }

    /// Check if this transition contains error data.
    pub fn has_error(&self) -> bool {
        matches!(self, Self::ToWithData(_, _))
    }
}

/// Shutdown mode for graceful or immediate termination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownMode {
    /// Graceful shutdown: process remaining events in the queue before terminating.
    Graceful,
    /// Immediate shutdown: terminate immediately without processing remaining events.
    Immediate,
}

/// Internal utility to parse durations using `humantime`.
pub fn parse_duration(s: &str) -> Result<Duration, humantime::DurationError> {
    s.parse::<humantime::Duration>().map(|d| d.into())
}
