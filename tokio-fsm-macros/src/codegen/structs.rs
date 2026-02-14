use proc_macro2::TokenStream;
use quote::quote;

use crate::validation::FsmStructure;

pub fn render_fsm_struct(fsm: &FsmStructure) -> TokenStream {
    let fsm_name = &fsm.fsm_name;
    let state_enum_name = fsm.state_enum_ident();
    let context_type = &fsm.context_type;

    quote! {
        /// The finite state machine structure.
        pub struct #fsm_name {
            state: #state_enum_name,
            context: #context_type,
        }
    }
}

pub fn render_handle_struct(fsm: &FsmStructure) -> TokenStream {
    let handle_name = fsm.handle_ident();
    let event_enum_name = fsm.event_enum_ident();
    let state_enum_name = fsm.state_enum_ident();

    quote! {
        /// A handle to the running FSM for event submission and state observation.
        #[derive(Clone)]
        pub struct #handle_name {
            event_tx: tokio::sync::mpsc::Sender<#event_enum_name>,
            state_rx: tokio::sync::watch::Receiver<#state_enum_name>,
            shutdown_tx: std::sync::Arc<tokio::sync::watch::Sender<Option<tokio_fsm_core::ShutdownMode>>>,
        }
    }
}

pub fn render_task_struct(fsm: &FsmStructure) -> TokenStream {
    let task_name = fsm.task_ident();
    let context_type = &fsm.context_type;
    let error_type = &fsm.error_type;

    quote! {
        /// A handle to the background task running the FSM.
        /// Awaiting this will return the final context or an error.
        pub struct #task_name {
            handle: tokio::task::JoinHandle<Result<#context_type, #error_type>>,
        }
    }
}
