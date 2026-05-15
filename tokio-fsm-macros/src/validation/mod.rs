//! Validation and semantic analysis for FSM structure.

mod graph;
mod parser;
mod types;

use quote::format_ident;
use syn::Ident;
pub use types::*;

impl FsmStructure {
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

    pub fn command_enum_ident(&self) -> Ident {
        format_ident!("{}Command", self.fsm_name)
    }
}
