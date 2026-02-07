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

/// Parses a duration string like "30s", "5m", "1h" into a `Duration`.
///
/// Supported units: `s` (seconds), `m` (minutes), `h` (hours), `ms` (milliseconds).
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty duration string".to_string());
    }

    let split_idx = s
        .char_indices()
        .find(|(_, c)| c.is_ascii_alphabetic())
        .map(|(idx, _)| idx)
        .ok_or_else(|| format!("No unit found in duration string: '{s}'"))?;

    let (num_str, unit) = s.split_at(split_idx);
    let num: u64 = num_str
        .parse()
        .map_err(|_| format!("Invalid number in duration string: '{num_str}'"))?;

    match unit {
        "ms" => Ok(Duration::from_millis(num)),
        "s" => Ok(Duration::from_secs(num)),
        "m" => Ok(Duration::from_secs(num * 60)),
        "h" => Ok(Duration::from_secs(num * 3600)),
        _ => Err(format!(
            "Unknown duration unit: '{unit}' (expected ms, s, m, or h)"
        )),
    }
}
