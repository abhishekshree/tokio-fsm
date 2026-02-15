use proc_macro2::TokenStream;
use quote::quote;

use crate::validation::FsmStructure;

pub fn render_spawn(fsm: &FsmStructure) -> TokenStream {
    let fsm_name = &fsm.fsm_name;
    let handle_name = fsm.handle_ident();
    let task_name = fsm.task_ident();
    let state_enum_name = fsm.state_enum_ident();
    let initial_state = &fsm.initial_state;
    let channel_size = fsm.channel_size;
    let context_type = &fsm.context_type;

    quote! {
        pub fn spawn(context: #context_type) -> (#handle_name, #task_name) {
            let (event_tx, event_rx) = tokio::sync::mpsc::channel(#channel_size);
            let (state_tx, state_rx) = tokio::sync::watch::channel(#state_enum_name::#initial_state);
            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(None);

            let fsm = #fsm_name {
                state: #state_enum_name::#initial_state,
                context,
            };

            let shutdown_tx = std::sync::Arc::new(shutdown_tx);
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

pub fn render_run(fsm: &FsmStructure) -> TokenStream {
    let event_enum_name = fsm.event_enum_ident();
    let state_enum_name = fsm.state_enum_ident();
    let context_type = &fsm.context_type;
    let error_type = &fsm.error_type;

    let event_arms = build_event_arms(fsm);
    let timeout_logic = build_timeout_handler(fsm);

    quote! {
        async fn run(
            mut self,
            mut events: tokio::sync::mpsc::Receiver<#event_enum_name>,
            mut shutdown: tokio::sync::watch::Receiver<Option<tokio_fsm::ShutdownMode>>,
            state_tx: tokio::sync::watch::Sender<#state_enum_name>,
        ) -> Result<#context_type, #error_type> {
            let sleep = tokio::time::sleep(tokio::time::Duration::from_secs(3153600000));
            tokio::pin!(sleep);

            loop {
                tokio::select! {
                    _ = &mut sleep => {
                        #timeout_logic
                        sleep.as_mut().reset(tokio::time::Instant::now() + tokio::time::Duration::from_secs(3153600000));
                    }
                    _ = shutdown.changed() => {
                        let mode = *shutdown.borrow();
                        if let Some(mode) = mode {
                            match mode {
                                tokio_fsm::ShutdownMode::Immediate => return Ok(self.context),
                                tokio_fsm::ShutdownMode::Graceful => {
                                    while let Ok(event) = events.try_recv() {
                                         match (self.state, event) {
                                            #(#event_arms)*
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
                            #(#event_arms)*
                            _ => {
                                // Event not handled in current state — silently ignored
                            }
                        }
                    }
                }
            }

            Ok(self.context)
        }
    }
}

pub fn render_handle_impl(fsm: &FsmStructure) -> TokenStream {
    let handle_name = fsm.handle_ident();
    let event_enum_name = fsm.event_enum_ident();
    let state_enum_name = fsm.state_enum_ident();

    quote! {
        impl #handle_name {
            /// Sends an event to the FSM.
            pub async fn send(&self, event: #event_enum_name) -> Result<(), tokio::sync::mpsc::error::SendError<#event_enum_name>> {
                self.event_tx.send(event).await
            }

            /// Attempts to send an event without awaiting capacity.
            pub fn try_send(&self, event: #event_enum_name) -> Result<(), tokio::sync::mpsc::error::TrySendError<#event_enum_name>> {
                self.event_tx.try_send(event)
            }

            /// Returns the current state of the FSM.
            pub fn current_state(&self) -> #state_enum_name {
                *self.state_rx.borrow()
            }

            /// Waits for the FSM to reach the specified state.
            pub async fn wait_for_state(&self, target: #state_enum_name) -> Result<(), tokio::sync::watch::error::RecvError> {
                let mut rx = self.state_rx.clone();
                while *rx.borrow_and_update() != target {
                    rx.changed().await?;
                }
                Ok(())
            }

            /// Initiates a graceful shutdown. Processes remaining events before exiting.
            pub fn shutdown_graceful(&self) {
                let _ = self.shutdown_tx.send(Some(tokio_fsm::ShutdownMode::Graceful));
            }

            /// Initiates an immediate shutdown. Drops unprocessed events.
            pub fn shutdown_immediate(&self) {
                let _ = self.shutdown_tx.send(Some(tokio_fsm::ShutdownMode::Immediate));
            }
        }
    }
}

pub fn render_task_impl(fsm: &FsmStructure) -> TokenStream {
    let task_name = fsm.task_ident();
    let context_type = &fsm.context_type;
    let error_type = &fsm.error_type;

    quote! {
        impl std::future::Future for #task_name {
            type Output = Result<#context_type, tokio_fsm::TaskError<#error_type>>;

            fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                match std::pin::Pin::new(&mut self.handle).poll(cx) {
                    std::task::Poll::Ready(Ok(Ok(res))) => std::task::Poll::Ready(Ok(res)),
                    std::task::Poll::Ready(Ok(Err(e))) => std::task::Poll::Ready(Err(tokio_fsm::TaskError::Fsm(e))),
                    std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(tokio_fsm::TaskError::Join(e))),
                    std::task::Poll::Pending => std::task::Poll::Pending,
                }
            }
        }
    }
}

// --- Event loop logic (previously in logic.rs) ---

/// Builds state-gated match arms for the event loop.
fn build_event_arms(fsm: &FsmStructure) -> Vec<TokenStream> {
    let mut arms = Vec::new();
    let event_enum = fsm.event_enum_ident();
    let state_enum = fsm.state_enum_ident();

    for handler in &fsm.handlers {
        if let Some(ref event) = handler.event {
            let event_name = &event.name;
            let method_name = &handler.method.sig.ident;

            // Timeout reset logic
            let timeout_reset = if let Some(duration) = handler.timeout {
                let secs = duration.as_secs();
                let nanos = duration.subsec_nanos();
                quote! {
                    sleep.as_mut().reset(tokio::time::Instant::now() + std::time::Duration::new(#secs, #nanos));
                }
            } else {
                quote! {
                    sleep.as_mut().reset(tokio::time::Instant::now() + std::time::Duration::from_secs(3153600000));
                }
            };

            // Payload handling
            let (payload_pattern, payload_call) = if handler.has_payload {
                (quote! { (payload) }, quote! { (payload) })
            } else {
                (quote! {}, quote! { () })
            };

            // Result vs direct transition
            let arm_inner = if handler.is_result {
                quote! {
                    match self.#method_name #payload_call .await {
                        Ok(transition) => {
                            self.state = transition.into_state().into();
                            let _ = state_tx.send(self.state);
                            #timeout_reset
                        }
                        Err(transition) => {
                            self.state = transition.into_state().into();
                            let _ = state_tx.send(self.state);
                            sleep.as_mut().reset(tokio::time::Instant::now() + std::time::Duration::from_secs(3153600000));
                        }
                    }
                }
            } else {
                quote! {
                    let transition = self.#method_name #payload_call .await;
                    self.state = transition.into_state().into();
                    let _ = state_tx.send(self.state);
                    #timeout_reset
                }
            };

            // Generate one match arm per source state (state-gated)
            for source_state in &handler.source_states {
                arms.push(quote! {
                    (#state_enum::#source_state, #event_enum::#event_name #payload_pattern) => {
                        #arm_inner
                    }
                });
            }
        }
    }

    arms
}

/// Builds the timeout handler block for the run loop.
fn build_timeout_handler(fsm: &FsmStructure) -> TokenStream {
    if let Some(handler) = fsm.handlers.iter().find(|h| h.is_timeout_handler) {
        let name = &handler.method.sig.ident;
        quote! {
            let transition = self.#name().await;
            self.state = transition.into_state().into();
            let _ = state_tx.send(self.state);
        }
    } else {
        quote! {}
    }
}
