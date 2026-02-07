//! Validation logic for FSM structure.

use syn::{Error, Ident, ImplItem, Type};
use std::collections::HashSet;
use darling::FromMeta;

use crate::attrs;

/// Represents a discovered state in the FSM.
#[derive(Debug, Clone)]
pub struct State {
    pub name: Ident,
}

/// Represents a discovered event in the FSM.
#[derive(Debug, Clone)]
pub struct Event {
    pub name: Ident,
    pub payload_type: Option<Type>,
}

/// Represents a handler method in the FSM.
#[derive(Debug, Clone)]
pub struct Handler {
    pub method: syn::ImplItemFn,
    pub event: Option<Event>,
    pub is_timeout_handler: bool,
    pub state_timeout: Option<attrs::StateTimeoutAttr>,
    pub return_states: Vec<State>,
}

/// Represents the complete FSM structure after parsing.
#[derive(Debug)]
pub struct FsmStructure {
    pub fsm_name: Ident,
    pub initial_state: Ident,
    pub channel_size: usize,
    pub context_type: Type,
    pub error_type: Type,
    pub states: Vec<State>,
    pub events: Vec<Event>,
    pub handlers: Vec<Handler>,
}

impl FsmStructure {
    /// Parse the impl block and extract FSM structure.
    pub fn parse(
        args: attrs::FsmArgs,
        impl_block: syn::ItemImpl,
    ) -> syn::Result<Self> {
        // Extract FSM name from impl block
        let fsm_name = match &*impl_block.self_ty {
            syn::Type::Path(path) => {
                path.path
                    .segments
                    .last()
                    .ok_or_else(|| Error::new_spanned(&impl_block.self_ty, "Expected FSM type name"))?
                    .ident
                    .clone()
            }
            _ => return Err(Error::new_spanned(&impl_block.self_ty, "Expected type path for FSM")),
        };

        // Extract initial state
        let initial_state = args.initial_ident()?;

        // Extract associated types
        let mut context_type = None;
        let mut error_type = None;

        for item in &impl_block.items {
            if let ImplItem::Type(ty) = item {
                if ty.ident == "Context" {
                    context_type = Some(ty.ty.clone());
                } else if ty.ident == "Error" {
                    error_type = Some(ty.ty.clone());
                }
            }
        }

        let context_type = context_type.ok_or_else(|| {
            Error::new_spanned(&impl_block, "Missing associated type: type Context = ...")
        })?;
        let error_type = error_type.ok_or_else(|| {
            Error::new_spanned(&impl_block, "Missing associated type: type Error = ...")
        })?;

        // Parse methods
        let mut handlers = Vec::new();
        let mut events = Vec::new();
        let mut states = HashSet::new();

        // Add initial state
        states.insert(initial_state.clone());

        for item in &impl_block.items {
            if let ImplItem::Fn(method) = item {
                let handler = Handler::parse(method)?;
                
                // Collect states from return types
                for state in &handler.return_states {
                    states.insert(state.name.clone());
                }

                // Collect events
                if let Some(ref event) = handler.event {
                    events.push(event.clone());
                }

                handlers.push(handler);
            }
        }

        let states: Vec<State> = states.into_iter().map(|name| State { name }).collect();

        Ok(Self {
            fsm_name,
            initial_state,
            channel_size: args.channel_size,
            context_type,
            error_type,
            states,
            events,
            handlers,
        })
    }
}

impl Handler {
    /// Parse a method into a Handler.
    fn parse(method: &syn::ImplItemFn) -> syn::Result<Self> {
        let mut event = None;
        let mut is_timeout_handler = false;
        let mut state_timeout = None;

        // Parse attributes
        for attr in &method.attrs {
            if attr.path().is_ident("event") {
                // Parse event attribute - it should be #[event(EventName)]
                let meta = &attr.meta;
                if let syn::Meta::List(list) = meta {
                    if let Ok(event_ident) = syn::parse2::<Ident>(list.tokens.clone()) {
                        // Extract event name and payload type from method signature
                        let payload_type = if method.sig.inputs.len() > 1 {
                            // Skip &mut self
                            if let syn::FnArg::Typed(pat_type) = &method.sig.inputs[1] {
                                Some((*pat_type.ty).clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        event = Some(Event {
                            name: event_ident,
                            payload_type,
                        });
                    }
                }
            } else if attr.path().is_ident("on_timeout") {
                is_timeout_handler = true;
            } else if attr.path().is_ident("state_timeout") {
                state_timeout = Some(attrs::StateTimeoutAttr::from_meta(
                    &attr.meta,
                )?);
            }
        }

        // Extract return states from return type
        let return_states = extract_return_states(&method.sig.output)?;

        Ok(Self {
            method: method.clone(),
            event,
            is_timeout_handler,
            state_timeout,
            return_states,
        })
    }
}

/// Extract state names from a return type (Transition<State> or Result<Transition<State>, Transition<State>>).
fn extract_return_states(output: &syn::ReturnType) -> syn::Result<Vec<State>> {
    let return_type = match output {
        syn::ReturnType::Type(_, ty) => ty.as_ref(),
        syn::ReturnType::Default => return Ok(Vec::new()),
    };

    let mut states = Vec::new();

    // Check for Result<Transition<State>, Transition<State>>
    if let Type::Path(path) = return_type {
        if let Some(segment) = path.path.segments.last() {
            if segment.ident == "Result" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    // Extract both Ok and Err variants
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(ty) = arg {
                            extract_states_from_transition(ty, &mut states)?;
                        }
                    }
                }
                return Ok(states);
            }
        }
    }

    // Check for Transition<State>
    extract_states_from_transition(return_type, &mut states)?;

    Ok(states)
}

/// Extract state names from Transition<State>.
fn extract_states_from_transition(ty: &Type, states: &mut Vec<State>) -> syn::Result<()> {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            if segment.ident == "Transition" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(ty) = arg {
                            if let Type::Path(state_path) = ty {
                                if let Some(state_seg) = state_path.path.segments.last() {
                                    states.push(State {
                                        name: state_seg.ident.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
