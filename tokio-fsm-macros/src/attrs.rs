//! Attribute parsing for FSM macro.

use darling::{Error, FromMeta, Result};
use syn::{Attribute, Ident, Token, bracketed, parse::ParseStream, punctuated::Punctuated};

/// Arguments for the `#[fsm]` attribute.
#[derive(Debug, FromMeta)]
pub struct FsmArgs {
    /// Initial state (required).
    pub initial: Ident,

    /// Channel size for event queue (default: 100). Must be greater than 0.
    #[darling(default = "default_channel_size", and_then = validate_channel_size)]
    pub channel_size: usize,

    /// Enable tracing support (default: false).
    #[darling(default)]
    pub tracing: bool,

    /// Enable serde support (default: false).
    #[darling(default)]
    pub serde: bool,
}

fn default_channel_size() -> usize {
    100
}

fn validate_channel_size(size: usize) -> Result<usize> {
    if size == 0 {
        return Err(Error::custom("channel_size must be greater than 0"));
    }
    Ok(size)
}

/// Arguments for the `#[on(state = Idle, event = Start, next = Running)]`
/// attribute.
#[derive(Debug)]
pub struct OnAttr {
    /// Source state this handler is valid in.
    pub state: Ident,
    /// Event that triggers this handler.
    pub event: Ident,
    /// States this handler may transition to.
    pub next: Vec<Ident>,
}

impl OnAttr {
    pub fn parse(attr: &Attribute) -> syn::Result<Self> {
        let mut state = None;
        let mut event = None;
        let mut next = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("state") {
                state = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("event") {
                event = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("next") {
                let input = meta.value()?;
                next = Some(parse_next(input)?);
                Ok(())
            } else {
                Err(meta.error("unsupported #[on] attribute key"))
            }
        })?;

        let next = next.ok_or_else(|| syn::Error::new_spanned(attr, "missing #[on] key: next"))?;
        if next.is_empty() {
            return Err(syn::Error::new_spanned(
                attr,
                "#[on] next list must contain at least one state",
            ));
        }

        Ok(Self {
            state: state
                .ok_or_else(|| syn::Error::new_spanned(attr, "missing #[on] key: state"))?,
            event: event
                .ok_or_else(|| syn::Error::new_spanned(attr, "missing #[on] key: event"))?,
            next,
        })
    }
}

fn parse_next(input: ParseStream<'_>) -> syn::Result<Vec<Ident>> {
    if input.peek(syn::token::Bracket) {
        let content;
        bracketed!(content in input);
        let values = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
        Ok(values.into_iter().collect())
    } else {
        Ok(vec![input.parse()?])
    }
}
