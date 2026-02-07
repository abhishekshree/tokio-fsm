//! Core runtime types for tokio-fsm.
//!
//! This crate provides the fundamental types used by generated finite state machines,
//! including transitions, shutdown modes, and state observation primitives.

use std::time::Duration;

/// Represents a state transition in the FSM.
///
/// Transitions can be either successful (moving to a new state) or error-based
/// (moving to a state with associated error data).
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
///
/// # Examples
///
/// ```
/// use tokio_fsm_core::parse_duration;
/// use std::time::Duration;
///
/// assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
/// assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
/// assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
/// assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
/// ```
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty duration string".to_string());
    }

    // Find the split point between number and unit
    let split_idx = s
        .char_indices()
        .rfind(|(_, c)| c.is_ascii_alphabetic())
        .map(|(idx, _)| idx + 1)
        .ok_or_else(|| format!("No unit found in duration string: {}", s))?;

    let (num_str, unit) = s.split_at(split_idx);
    let num: u64 = num_str
        .parse()
        .map_err(|_| format!("Invalid number in duration string: {}", num_str))?;

    let duration = match unit {
        "ms" => Duration::from_millis(num),
        "s" => Duration::from_secs(num),
        "m" => Duration::from_secs(num * 60),
        "h" => Duration::from_secs(num * 3600),
        _ => return Err(format!("Unknown duration unit: {}", unit)),
    };

    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_to() {
        let t = Transition::to(42);
        assert!(!t.has_error());
        assert_eq!(t.into_state(), 42);
    }

    #[test]
    fn test_transition_to_with_data() {
        let err = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        let t = Transition::to_with_data(42, err);
        assert!(t.has_error());
        assert_eq!(t.into_state(), 42);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert!(parse_duration("invalid").is_err());
        assert!(parse_duration("").is_err());
    }
}

