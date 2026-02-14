use quote::format_ident;
use syn::Ident;

/// Generates the identifier for the FSM's state enum: `[FsmName]State`
pub fn state_enum_ident(fsm_name: &Ident) -> Ident {
    format_ident!("{}State", fsm_name)
}

/// Generates the identifier for the FSM's event enum: `[FsmName]Event`
pub fn event_enum_ident(fsm_name: &Ident) -> Ident {
    format_ident!("{}Event", fsm_name)
}

/// Generates the identifier for the FSM's handle struct: `[FsmName]Handle`
pub fn handle_ident(fsm_name: &Ident) -> Ident {
    format_ident!("{}Handle", fsm_name)
}

/// Generates the identifier for the FSM's task struct: `[FsmName]Task`
pub fn task_ident(fsm_name: &Ident) -> Ident {
    format_ident!("{}Task", fsm_name)
}
