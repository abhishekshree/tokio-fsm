//! Validation and semantic analysis for FSM structure.
//!
//! This module is the first layer of the macro pipeline. It:
//! 1. Parses the `impl` block to extract states, events, and handlers
//! 2. Derives semantic fields (timeout durations, payload presence, result types)
//! 3. Validates the FSM graph (reachability from initial state)

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use darling::FromMeta;
use petgraph::{algo::has_path_connecting, graph::DiGraph};
use quote::format_ident;
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

/// Represents a handler method in the FSM, including all derived semantic fields.
#[derive(Debug, Clone)]
pub struct Handler {
    pub method: syn::ImplItemFn,
    pub event: Option<Event>,
    pub is_timeout_handler: bool,
    pub return_states: Vec<State>,

    // Derived semantic fields (previously in IR)
    /// Source states this handler is valid in.
    pub source_states: Vec<Ident>,
    /// Whether the event carries a payload argument.
    pub has_payload: bool,
    /// Whether the return type is `Result<Transition<A>, Transition<B>>`.
    pub is_result: bool,
    /// Parsed timeout duration for the target state, if any.
    pub timeout: Option<Duration>,
}

/// The complete FSM structure after parsing and validation.
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
    // --- Ident helpers (previously in helpers.rs) ---

    pub fn state_enum_ident(&self) -> Ident {
        format_ident!("{}State", self.fsm_name)
    }

    pub fn event_enum_ident(&self) -> Ident {
        format_ident!("{}Event", self.fsm_name)
    }

    pub fn handle_ident(&self) -> Ident {
        format_ident!("{}Handle", self.fsm_name)
    }

    pub fn task_ident(&self) -> Ident {
        format_ident!("{}Task", self.fsm_name)
    }

    // --- Parsing ---

    /// Parse the impl block and extract the complete FSM structure.
    pub fn parse(args: attrs::FsmArgs, impl_block: &syn::ItemImpl) -> syn::Result<Self> {
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
            Error::new_spanned(impl_block, "Missing associated type: type Context = ...")
        })?;
        let error_type = error_type.ok_or_else(|| {
            Error::new_spanned(impl_block, "Missing associated type: type Error = ...")
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

                // Collect source states
                for state in &handler.source_states {
                    states_set.insert(state.clone());
                }

                // Collect events
                if let Some(ref event) = handler.event
                    && !event_names.contains(&event.name)
                {
                    event_names.insert(event.name.clone());
                    events.push(event.clone());
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

    /// Validate the FSM graph for reachability.
    ///
    /// Constructs a directed graph where:
    /// - **Nodes**: FSM states
    /// - **Edges**: Transitions from declared source states to return states
    ///
    /// Checks:
    /// 1. All declared states exist as nodes
    /// 2. All states are reachable from the initial state
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
            for target in &handler.return_states {
                let target_node = nodes.get(&target.name).ok_or_else(|| {
                    syn::Error::new_spanned(
                        &target.name,
                        format!(
                            "Target state '{}' not found in discovered states",
                            target.name
                        ),
                    )
                })?;

                if handler.source_states.is_empty() {
                    // Timeout handlers have no source states — they can fire from any
                    // state that has a timeout. Add edges from all states.
                    for &source_node in nodes.values() {
                        graph.add_edge(source_node, *target_node, ());
                    }
                } else {
                    // State-gated: add edges only from declared source states
                    for source_ident in &handler.source_states {
                        let source_node = nodes.get(source_ident).ok_or_else(|| {
                            syn::Error::new_spanned(
                                source_ident,
                                format!(
                                    "Source state '{}' in #[state(...)] not found in FSM states",
                                    source_ident
                                ),
                            )
                        })?;
                        graph.add_edge(*source_node, *target_node, ());
                    }
                }
            }
        }

        // Check reachability from initial state to all other states
        for (&state_name, &node) in &nodes {
            if !has_path_connecting(&graph, *initial_node, node, None) {
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
    /// Parse a method into a Handler with all semantic fields derived.
    fn parse(method: &syn::ImplItemFn) -> syn::Result<Self> {
        let mut event = None;
        let mut is_timeout_handler = false;
        let mut state_timeout_attr = None;
        let mut source_states = Vec::new();

        // Parse attributes
        for attr in &method.attrs {
            if attr.path().is_ident("event") {
                let attr_args: attrs::EventAttr = attrs::EventAttr::from_meta(&attr.meta)?;
                let payload_type = if method.sig.inputs.len() > 1 {
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
                state_timeout_attr = Some(attrs::StateTimeoutAttr::from_meta(&attr.meta)?);
            } else if attr.path().is_ident("state") {
                let state_attr: attrs::StateAttr = attrs::StateAttr::from_meta(&attr.meta)?;
                source_states = state_attr.states;
            }
        }

        // Validate: event handlers must have #[state(...)]
        if event.is_some() && source_states.is_empty() {
            return Err(syn::Error::new_spanned(
                &method.sig.ident,
                "Event handlers require #[state(StateName, ...)] to declare valid source states",
            ));
        }

        // Derive: has_payload
        let has_payload = event
            .as_ref()
            .map(|e| e.payload_type.is_some())
            .unwrap_or(false);

        // Derive: is_result
        let is_result = match &method.sig.output {
            syn::ReturnType::Type(_, ty) => {
                if let syn::Type::Path(path) = ty.as_ref() {
                    path.path
                        .segments
                        .last()
                        .map(|seg| seg.ident == "Result")
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            syn::ReturnType::Default => false,
        };

        // Derive: timeout (fail loudly on invalid duration)
        let timeout = if let Some(ref st) = state_timeout_attr {
            let duration_str = st.duration.value();
            let parsed = humantime::parse_duration(&duration_str).map_err(|e| {
                syn::Error::new_spanned(
                    &st.duration,
                    format!("Invalid duration '{}': {}", duration_str, e),
                )
            })?;
            Some(parsed)
        } else {
            None
        };

        // Extract return states from return type
        let return_states = extract_return_states(&method.sig.output)?;

        Ok(Self {
            method: method.clone(),
            event,
            is_timeout_handler,
            return_states,
            source_states,
            has_payload,
            is_result,
            timeout,
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
    if let Type::Path(path) = ty
        && let Some(segment) = path.path.segments.last()
    {
        if segment.ident == "Transition" {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                for arg in &args.args {
                    if let GenericArgument::Type(Type::Path(inner_path)) = arg
                        && let Some(state_seg) = inner_path.path.segments.last()
                    {
                        states.push(State {
                            name: state_seg.ident.clone(),
                        });
                    }
                }
            }
        } else if segment.ident == "Result"
            && let PathArguments::AngleBracketed(args) = &segment.arguments
        {
            for arg in &args.args {
                if let GenericArgument::Type(inner_ty) = arg {
                    extract_states_recursive(inner_ty, states)?;
                }
            }
        }
    }
    Ok(())
}
