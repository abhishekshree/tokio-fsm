use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::FsmIr;

pub fn render_spawn(ir: &FsmIr) -> TokenStream {
    let fsm_name = &ir.fsm_name;
    let handle_name = &ir.handle_name;
    let task_name = &ir.task_name;
    let state_enum_name = &ir.state_enum_ident;
    let initial_state = &ir.initial_state;
    let channel_size = ir.channel_size;
    let context_type = &ir.context_type;

    quote! {
        pub fn spawn(context: #context_type) -> (#handle_name, #task_name) {
            let (event_tx, event_rx) = tokio::sync::mpsc::channel(#channel_size);
            let (state_tx, state_rx) = tokio::sync::watch::channel(#state_enum_name::#initial_state);
            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(None);

            let fsm = #fsm_name {
                state: #state_enum_name::#initial_state,
                context,
            };

            let handle = tokio::spawn(fsm.run(event_rx, shutdown_rx, state_tx));

            (
                #handle_name {
                    event_tx,
                    state_rx,
                    shutdown_tx,
                },
                #task_name { handle },
            )
        }
    }
}

pub fn render_run(
    ir: &FsmIr,
    event_match_arms: &[TokenStream],
    timeout_logic: &TokenStream,
) -> TokenStream {
    let event_enum_name = &ir.event_enum_ident;
    let state_enum_name = &ir.state_enum_ident;
    let context_type = &ir.context_type;
    let error_type = &ir.error_type;

    quote! {
        async fn run(
            mut self,
            mut events: tokio::sync::mpsc::Receiver<#event_enum_name>,
            mut shutdown: tokio::sync::watch::Receiver<Option<tokio_fsm_core::ShutdownMode>>,
            state_tx: tokio::sync::watch::Sender<#state_enum_name>,
        ) -> Result<#context_type, #error_type> {
            let mut timeout: Option<std::pin::Pin<Box<tokio::time::Sleep>>> = None;

            loop {
                // If we have a timeout, we need to select on it
                if let Some(timer) = &mut timeout {
                    tokio::select! {
                        _ = timer => {
                            #timeout_logic
                            // Clear timeout after it fires
                            timeout = None;
                        }
                        // Important: Check shutdown BEFORE events to allow immediate exit
                        _ = shutdown.changed() => {
                            let mode = *shutdown.borrow();
                            if let Some(mode) = mode {
                                match mode {
                                    tokio_fsm_core::ShutdownMode::Immediate => return Ok(self.context),
                                    tokio_fsm_core::ShutdownMode::Graceful => {
                                        // Process remaining events then exit
                                        while let Ok(event) = events.try_recv() {
                                             match (self.state, event) {
                                                #(#event_match_arms)*
                                                _ => {} // Ignore unexpected events during shutdown or normal op
                                            }
                                        }
                                        return Ok(self.context);
                                    }
                                }
                            }
                        }
                        event = events.recv() => {
                            let Some(event) = event else { break };
                            match (self.state, event) {
                                #(#event_match_arms)*
                                _ => {
                                    // Event not handled in current state, ignore
                                }
                            }
                        }
                    }
                } else {
                    // No timeout, simpler select
                     tokio::select! {
                        _ = shutdown.changed() => {
                            let mode = *shutdown.borrow();
                             if let Some(mode) = mode {
                                match mode {
                                    tokio_fsm_core::ShutdownMode::Immediate => return Ok(self.context),
                                    tokio_fsm_core::ShutdownMode::Graceful => {
                                        // Process remaining events
                                        while let Ok(event) = events.try_recv() {
                                             match (self.state, event) {
                                                #(#event_match_arms)*
                                                _ => {}
                                            }
                                        }
                                        return Ok(self.context);
                                    }
                                }
                            }
                        }
                        event = events.recv() => {
                            let Some(event) = event else { break };
                            match (self.state, event) {
                                #(#event_match_arms)*
                                _ => {
                                    // Event not handled in current state
                                }
                            }
                        }
                    }
                }
            }

            Ok(self.context)
        }
    }
}

pub fn render_handle_impl(ir: &FsmIr) -> TokenStream {
    let handle_name = &ir.handle_name;
    let event_enum_name = &ir.event_enum_ident;
    let state_enum_name = &ir.state_enum_ident;

    quote! {
        impl #handle_name {
            /// Sends an event to the FSM.
            pub async fn send(&self, event: #event_enum_name) -> Result<(), tokio::sync::mpsc::error::SendError<#event_enum_name>> {
                self.event_tx.send(event).await
            }

            /// Attempts to send an event to the FSM without awaiting capacity.
            pub fn try_send(&self, event: #event_enum_name) -> Result<(), tokio::sync::mpsc::error::TrySendError<#event_enum_name>> {
                self.event_tx.try_send(event)
            }

            /// Returns the current state of the FSM.
            pub fn current_state(&self) -> #state_enum_name {
                *self.state_rx.borrow()
            }

            /// Waits for the FSM to reach the specified state.
            pub async fn wait_for_state(&mut self, target: #state_enum_name) -> Result<(), tokio::sync::watch::error::RecvError> {
                while *self.state_rx.borrow() != target {
                    self.state_rx.changed().await?;
                }
                Ok(())
            }

            /// Initiates a graceful shutdown of the FSM.
            /// The FSM will process all remaining events in the queue before exiting.
            pub fn shutdown_graceful(&self) {
                let _ = self.shutdown_tx.send(Some(tokio_fsm_core::ShutdownMode::Graceful));
            }

            /// Initiates an immediate shutdown of the FSM.
            /// The FSM will exit immediately, dropping any unprocessed events.
            pub fn shutdown_immediate(&self) {
                let _ = self.shutdown_tx.send(Some(tokio_fsm_core::ShutdownMode::Immediate));
            }
        }
    }
}

pub fn render_task_impl(ir: &FsmIr) -> TokenStream {
    let task_name = &ir.task_name;
    let context_type = &ir.context_type;
    let error_type = &ir.error_type;

    quote! {
        impl std::future::Future for #task_name {
            type Output = Result<#context_type, #error_type>;

            fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                use std::task::Poll;
                match std::pin::Pin::new(&mut self.handle).poll(cx) {
                    Poll::Ready(Ok(res)) => Poll::Ready(res),
                    Poll::Ready(Err(e)) => {
                        // JoinError - task panicked or cancelled
                        // We can't really return the user's error type here easily unless we wrap JoinError
                        // For now, we panic if the task panicked to propagate it.
                        if e.is_panic() {
                            std::panic::resume_unwind(e.into_panic());
                        } else {
                            // Cancelled
                            Poll::Pending // Or error?
                        }
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}
