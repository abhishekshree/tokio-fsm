use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::FsmIr;

pub fn render_fsm_struct(ir: &FsmIr) -> TokenStream {
    let fsm_name = &ir.fsm_name;
    let state_enum_name = &ir.state_enum_ident;
    let context_type = &ir.context_type;

    quote! {
        /// The finite state machine structure.
        pub struct #fsm_name {
            state: #state_enum_name,
            context: #context_type,
        }
    }
}

pub fn render_handle_struct(ir: &FsmIr) -> TokenStream {
    let handle_name = &ir.handle_name;
    let event_enum_name = &ir.event_enum_ident;
    let state_enum_name = &ir.state_enum_ident;

    quote! {
        /// A handle to the running FSM, allowing for event submission and state monitoring.
        #[derive(Clone)]
        pub struct #handle_name {
            event_tx: tokio::sync::mpsc::Sender<#event_enum_name>,
            state_rx: tokio::sync::watch::Receiver<#state_enum_name>,
            shutdown_tx: tokio::sync::watch::Sender<Option<tokio_fsm_core::ShutdownMode>>,
        }
    }
}

pub fn render_task_struct(ir: &FsmIr) -> TokenStream {
    let task_name = &ir.task_name;
    let context_type = &ir.context_type;
    let error_type = &ir.error_type;

    quote! {
        /// A handle to the background task running the FSM.
        /// Awaiting this will return the final context or an error.
        pub struct #task_name {
            handle: tokio::task::JoinHandle<Result<#context_type, #error_type>>,
        }
    }
}
