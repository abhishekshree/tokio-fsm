//! Attribute parsing for FSM macro.

use darling::{FromAttributes, FromMeta};
use syn::{parse_str, Ident, LitStr};

/// Arguments for the `#[fsm]` attribute.
#[derive(Debug)]
pub struct FsmArgs {
    /// Initial state name (required).
    pub initial: LitStr,
    
    /// Channel size for event queue (default: 100).
    pub channel_size: usize,
}

/// Arguments for the `#[event]` attribute.
#[derive(Debug, FromAttributes)]
#[darling(attributes(event))]
pub struct EventAttr {
    /// Event name (required).
    pub event: Ident,
}

/// Arguments for the `#[state_timeout]` attribute.
#[derive(Debug, Clone, FromMeta)]
pub struct StateTimeoutAttr {
    /// Duration string (e.g., "30s", "5m").
    #[darling(rename = "duration")]
    pub duration: LitStr,
}

/// Marker for `#[on_timeout]` attribute (no arguments).
#[derive(Debug, FromAttributes)]
#[darling(attributes(on_timeout))]
pub struct OnTimeoutAttr;

impl FsmArgs {
    /// Parse the initial state as an identifier.
    pub fn initial_ident(&self) -> syn::Result<Ident> {
        parse_str(&self.initial.value())
    }
}
