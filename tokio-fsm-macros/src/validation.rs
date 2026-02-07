//! Validation logic for FSM structure.

use std::collections::{HashMap, HashSet};

use darling::FromMeta;
use petgraph::{algo::has_path_connecting, graph::DiGraph};
use syn::{Error, FnArg, GenericArgument, Ident, ImplItem, PathArguments, ReturnType, Type};

use crate::attrs;

/// Represents a discovered state in the FSM.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    pub fn parse(args: attrs::FsmArgs, impl_block: syn::ItemImpl) -> syn::Result<Self> {
        // Extract FSM name from impl block
        let fsm_name = match &*impl_block.self_ty {
            syn::Type::Path(path) => path
                .path
                .segments
                .last()
                .ok_or_else(|| Error::new_spanned(&impl_block.self_ty, "Expected FSM type name"))?
                .ident
                .clone(),
            _ => {
                return Err(Error::new_spanned(
                    &impl_block.self_ty,
                    "Expected type path for FSM",
                ));
            }
        };

        let initial_state = args.initial_ident();

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
        let mut event_names = HashSet::new();
        let mut events = Vec::new();
        let mut states_set = HashSet::new();

        states_set.insert(initial_state.clone());

        for item in &impl_block.items {
            if let ImplItem::Fn(method) = item {
                let handler = Handler::parse(method)?;

                // Collect states from return types
                for state in &handler.return_states {
                    states_set.insert(state.name.clone());
                }

                // Collect events
                if let Some(ref event) = handler.event {
                    if !event_names.contains(&event.name) {
                        event_names.insert(event.name.clone());
                        events.push(event.clone());
                    }
                }

                handlers.push(handler);
            }
        }

        let states: Vec<State> = states_set
            .iter()
            .map(|name| State { name: name.clone() })
            .collect();

        let fsm = Self {
            fsm_name,
            initial_state,
            channel_size: args.channel_size,
            context_type,
            error_type,
            states,
            events,
            handlers,
        };

        fsm.validate()?;

        Ok(fsm)
    }

    /// Validate the FSM structure (graph reachability, etc.)
    fn validate(&self) -> syn::Result<()> {
        let mut graph = DiGraph::<&Ident, ()>::new();
        let mut nodes = HashMap::new();

        for state in &self.states {
            let node = graph.add_node(&state.name);
            nodes.insert(&state.name, node);
        }

        let initial_node = nodes.get(&self.initial_state).ok_or_else(|| {
            syn::Error::new_spanned(
                &self.initial_state,
                "Initial state not found in discovered states",
            )
        })?;

        for handler in &self.handlers {
            // For each handler, we assume it can be called from ANY state for now
            // Unless we implement state-specific event handlers in the future.
            // For MVP, if a handler returns Transition<Target>, it creates an edge from ALL
            // states to Target. This is a bit coarse but safe for reachability.
            for target in &handler.return_states {
                let target_node = nodes.get(&target.name).ok_or_else(|| {
                    syn::Error::new_spanned(
                        &target.name,
                        "Target state not found in discovered states",
                    )
                })?;

                for &source_node in nodes.values() {
                    graph.add_edge(source_node, *target_node, ());
                }
            }
        }

        // Check reachability from initial state to all other states
        for (&state_name, &node) in &nodes {
            if !has_path_connecting(&graph, *initial_node, node, None) {
                // Return a warning/error? For now, let's just make it a compile error if
                // unreachable. In a real lib, we might want to allow it but
                // warn.
                return Err(syn::Error::new_spanned(
                    state_name,
                    format!(
                        "State '{}' is unreachable from initial state '{}'",
                        state_name, self.initial_state
                    ),
                ));
            }
        }

        Ok(())
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
                let attr_args: attrs::EventAttr = attrs::EventAttr::from_meta(&attr.meta)?;
                // Extract event name and payload type from method signature
                let payload_type = if method.sig.inputs.len() > 1 {
                    // Skip &mut self
                    if let FnArg::Typed(pat_type) = &method.sig.inputs[1] {
                        Some((*pat_type.ty).clone())
                    } else {
                        None
                    }
                } else {
                    None
                };
                event = Some(Event {
                    name: attr_args.name,
                    payload_type,
                });
            } else if attr.path().is_ident("on_timeout") {
                is_timeout_handler = true;
            } else if attr.path().is_ident("state_timeout") {
                state_timeout = Some(attrs::StateTimeoutAttr::from_meta(&attr.meta)?);
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

/// Extract state names from a return type (Transition<State> or
/// Result<Transition<State>, Transition<State>>).
fn extract_return_states(output: &ReturnType) -> syn::Result<Vec<State>> {
    let return_type = match output {
        ReturnType::Type(_, ty) => ty.as_ref(),
        ReturnType::Default => return Ok(Vec::new()),
    };

    let mut states = Vec::new();
    extract_states_recursive(return_type, &mut states)?;
    Ok(states)
}

fn extract_states_recursive(ty: &Type, states: &mut Vec<State>) -> syn::Result<()> {
    match ty {
        Type::Path(path) => {
            if let Some(segment) = path.path.segments.last() {
                if segment.ident == "Transition" {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let GenericArgument::Type(Type::Path(inner_path)) = arg {
                                if let Some(state_seg) = inner_path.path.segments.last() {
                                    states.push(State {
                                        name: state_seg.ident.clone(),
                                    });
                                }
                            }
                        }
                    }
                } else if segment.ident == "Result" {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let GenericArgument::Type(inner_ty) = arg {
                                extract_states_recursive(inner_ty, states)?;
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
