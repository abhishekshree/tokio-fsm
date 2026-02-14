//! Attribute parsing for FSM macro.

use darling::FromMeta;
use syn::{Ident, LitStr};

/// Arguments for the `#[fsm]` attribute.
#[derive(Debug, FromMeta)]
pub struct FsmArgs {
    /// Initial state name (required).
    pub initial: String,

    /// Channel size for event queue (default: 100).
    #[darling(default = "default_channel_size")]
    pub channel_size: usize,
}

fn default_channel_size() -> usize {
    100
}

/// Arguments for the `#[event]` attribute.
#[derive(Debug)]
pub struct EventAttr {
    pub name: Ident,
}

impl FromMeta for EventAttr {
    fn from_meta(meta: &syn::Meta) -> Result<Self, darling::Error> {
        match meta {
            syn::Meta::List(list) => {
                let ident = syn::parse2::<Ident>(list.tokens.clone())
                    .map_err(|_| darling::Error::custom("Expected event name"))?;
                Ok(EventAttr { name: ident })
            }
            _ => Err(darling::Error::custom("Expected #[event(EventName)]")),
        }
    }
}

/// Arguments for the `#[state_timeout]` attribute.
#[derive(Debug, Clone, FromMeta)]
pub struct StateTimeoutAttr {
    /// Duration string (e.g., "30s", "5m").
    pub duration: LitStr,
}

/// Arguments for the `#[state(...)]` attribute.
/// Specifies which states a handler is valid in.
#[derive(Debug)]
pub struct StateAttr {
    pub states: Vec<Ident>,
}

impl FromMeta for StateAttr {
    fn from_meta(meta: &syn::Meta) -> Result<Self, darling::Error> {
        match meta {
            syn::Meta::List(list) => {
                let parser = syn::punctuated::Punctuated::<Ident, syn::Token![,]>::parse_terminated;
                let punct =
                    syn::parse::Parser::parse2(parser, list.tokens.clone()).map_err(|_| {
                        darling::Error::custom("Expected state names: #[state(Idle, Running)]")
                    })?;
                let states: Vec<Ident> = punct.into_iter().collect();
                if states.is_empty() {
                    return Err(darling::Error::custom(
                        "#[state(...)] requires at least one state name",
                    ));
                }
                Ok(StateAttr { states })
            }
            _ => Err(darling::Error::custom("Expected #[state(StateName, ...)]")),
        }
    }
}

impl FsmArgs {
    /// Parse the initial state as an identifier.
    pub fn initial_ident(&self) -> Ident {
        Ident::new(&self.initial, proc_macro2::Span::call_site())
    }
}
