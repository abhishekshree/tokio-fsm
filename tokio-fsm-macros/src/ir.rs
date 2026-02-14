use std::time::Duration;

use syn::{Ident, Type};

use crate::helpers;
use crate::validation::FsmStructure;

/// Complete Intermediate Representation of the FSM.
/// This structure contains all the semantic decisions needed to generate code.
pub struct FsmIr {
    pub fsm_name: Ident,
    pub state_enum_ident: Ident,
    pub event_enum_ident: Ident,
    pub handle_name: Ident,
    pub task_name: Ident,

    pub context_type: Type,
    pub error_type: Type,

    pub initial_state: Ident,
    pub channel_size: usize,

    pub states: Vec<StateIr>,
    pub events: Vec<EventIr>,
    pub handlers: Vec<HandlerIr>,
}

pub struct StateIr {
    pub name: Ident,
}

pub struct EventIr {
    pub name: Ident,
    pub payload_type: Option<Type>,
}

pub struct HandlerIr {
    pub method_name: Ident,
    pub event_variant: Option<Ident>,
    pub has_payload: bool,
    pub is_async_result: bool,
    pub timeout: Option<Duration>,
    #[allow(dead_code)]
    pub return_states: Vec<Ident>,
    pub is_timeout_handler: bool,
}

impl From<&FsmStructure> for FsmIr {
    fn from(fsm: &FsmStructure) -> Self {
        let state_enum_ident = helpers::state_enum_ident(&fsm.fsm_name);
        let event_enum_ident = helpers::event_enum_ident(&fsm.fsm_name);
        let handle_name = helpers::handle_ident(&fsm.fsm_name);
        let task_name = helpers::task_ident(&fsm.fsm_name);

        let states = fsm
            .states
            .iter()
            .map(|s| StateIr {
                name: s.name.clone(),
            })
            .collect();

        let events = fsm
            .events
            .iter()
            .map(|e| EventIr {
                name: e.name.clone(),
                payload_type: e.payload_type.clone(),
            })
            .collect();

        let handlers = fsm
            .handlers
            .iter()
            .map(|h| {
                let event_variant = h.event.as_ref().map(|e| e.name.clone());
                let has_payload = h
                    .event
                    .as_ref()
                    .map(|e| e.payload_type.is_some())
                    .unwrap_or(false);

                // Semantic decision: Is the return type Result<Transition, ...>?
                let is_async_result = match &h.method.sig.output {
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

                // Semantic decision: Parse timeout duration
                let timeout = if let Some(ref st) = h.state_timeout {
                    let duration_str = st.duration.value();
                    humantime::parse_duration(&duration_str).ok()
                    // If parsing fails, we could panic here or return None.
                    // Ideally validation.rs catches invalid strings, but for now we consume valid ones.
                } else {
                    None
                };

                HandlerIr {
                    method_name: h.method.sig.ident.clone(),
                    event_variant,
                    has_payload,
                    is_async_result,
                    timeout,
                    return_states: h.return_states.iter().map(|s| s.name.clone()).collect(),
                    is_timeout_handler: h.is_timeout_handler,
                }
            })
            .collect();

        Self {
            fsm_name: fsm.fsm_name.clone(),
            state_enum_ident,
            event_enum_ident,
            handle_name,
            task_name,
            context_type: fsm.context_type.clone(),
            error_type: fsm.error_type.clone(),
            initial_state: fsm.initial_state.clone(),
            channel_size: fsm.channel_size,
            states,
            events,
            handlers,
        }
    }
}
