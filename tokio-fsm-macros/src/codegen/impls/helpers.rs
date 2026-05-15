//! Shared TokenStream builders and private FSM helper generation.

use proc_macro2::TokenStream;
use quote::quote;

use crate::validation::FsmStructure;

pub fn render_fsm_private_helpers(fsm: &FsmStructure) -> TokenStream {
    let state_enum_name = fsm.state_enum_ident();

    let tracing_log = if fsm.tracing {
        quote! {
            ::tokio_fsm::tracing::info!(
                from = ?old_state,
                to = ?self.state,
                event = event_name,
                "Transition successful"
            );
        }
    } else {
        quote! {
            let _ = event_name;
        }
    };

    quote! {
        fn apply_transition(
            &mut self,
            next: #state_enum_name,
            state_tx: &::tokio_fsm::tokio::sync::watch::Sender<#state_enum_name>,
            event_name: &str,
        ) -> #state_enum_name {
            let old_state = self.state;
            self.state = next;
            let _ = state_tx.send(self.state);
            #tracing_log
            self.state
        }
    }
}
