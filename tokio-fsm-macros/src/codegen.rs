//! Code generation for FSM implementation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::validation::FsmStructure;

/// Generate the complete FSM implementation.
pub fn generate(fsm: &FsmStructure, original_impl: &syn::ItemImpl) -> TokenStream {
    let state_enum = generate_state_enum(fsm);
    let event_enum = generate_event_enum(fsm);
    let fsm_struct = generate_fsm_struct(fsm);
    let handle_struct = generate_handle_struct(fsm);
    let task_struct = generate_task_struct(fsm);
    let spawn_impl = generate_spawn_impl(fsm);
    let run_impl = generate_run_impl(fsm);
    let handle_impl = generate_handle_impl(fsm);

    // Keep the original methods from the impl block
    let original_methods: Vec<_> = original_impl.items.iter()
        .filter_map(|item| {
            if let syn::ImplItem::Fn(method) = item {
                Some(&method.attrs)
                    .filter(|attrs| {
                        // Only keep methods that aren't event handlers or timeout handlers
                        !attrs.iter().any(|attr| {
                            attr.path().is_ident("event") || 
                            attr.path().is_ident("on_timeout") ||
                            attr.path().is_ident("state_timeout")
                        })
                    })
                    .map(|_| method)
            } else {
                None
            }
        })
        .collect();

    quote! {
        #state_enum
        #event_enum
        #fsm_struct
        #handle_struct
        #task_struct

        impl #fsm_struct {
            #spawn_impl
            #run_impl

            // Original user methods
            #(#original_methods)*
        }

        #handle_impl
    }
}

/// Generate the State enum.
fn generate_state_enum(fsm: &FsmStructure) -> TokenStream {
    let states: Vec<_> = fsm.states.iter().map(|s| &s.name).collect();
    
    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum State {
            #(#states,)*
        }
    }
}

/// Generate the Event enum.
fn generate_event_enum(fsm: &FsmStructure) -> TokenStream {
    let variants: Vec<TokenStream> = fsm.events.iter().map(|event| {
        let event_name = &event.name;
        if let Some(ref payload_type) = event.payload_type {
            quote! {
                #event_name(#payload_type),
            }
        } else {
            quote! {
                #event_name,
            }
        }
    }).collect();

    quote! {
        #[derive(Debug, Clone)]
        pub enum Event {
            #(#variants)*
        }
    }
}

/// Generate the FSM struct.
fn generate_fsm_struct(fsm: &FsmStructure) -> TokenStream {
    let fsm_name = &fsm.fsm_name;
    let context_type = &fsm.context_type;

    quote! {
        pub struct #fsm_name {
            state: State,
            context: #context_type,
        }
    }
}

/// Generate the Handle struct.
fn generate_handle_struct(fsm: &FsmStructure) -> TokenStream {
    let handle_name = format_ident!("{}Handle", fsm.fsm_name);

    quote! {
        pub struct #handle_name {
            event_tx: tokio::sync::mpsc::Sender<Event>,
            shutdown_tx: tokio::sync::watch::Sender<Option<tokio_fsm_core::ShutdownMode>>,
            state_rx: tokio::sync::watch::Receiver<State>,
        }
    }
}

/// Generate the Task struct.
fn generate_task_struct(fsm: &FsmStructure) -> TokenStream {
    let task_name = format_ident!("{}Task", fsm.fsm_name);
    let context_type = &fsm.context_type;
    let error_type = &fsm.error_type;

    quote! {
        #[must_use = "FSM task must be awaited or it will abort"]
        pub struct #task_name {
            handle: tokio::task::JoinHandle<Result<#context_type, #error_type>>,
        }

        impl #task_name {
            pub async fn await_task(self) -> Result<#context_type, #error_type> {
                self.handle.await.map_err(|e| {
                    // Convert JoinError to user error type if possible
                    // For now, we'll need to handle this based on user's Error type
                    panic!("FSM task panicked: {:?}", e)
                })?
            }
        }
    }
}

/// Generate the spawn implementation.
fn generate_spawn_impl(fsm: &FsmStructure) -> TokenStream {
    let fsm_name = &fsm.fsm_name;
    let handle_name = format_ident!("{}Handle", fsm_name);
    let task_name = format_ident!("{}Task", fsm_name);
    let context_type = &fsm.context_type;
    let initial_state = &fsm.initial_state;
    let channel_size = fsm.channel_size;

    quote! {
        pub fn spawn(context: #context_type) -> (#fsm_name, #handle_name, #task_name) {
            let (event_tx, event_rx) = tokio::sync::mpsc::channel(#channel_size);
            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(None);
            let (state_tx, state_rx) = tokio::sync::watch::channel(State::#initial_state);

            let fsm = #fsm_name {
                state: State::#initial_state,
                context,
            };

            let handle = tokio::spawn(async move {
                fsm.run(event_rx, shutdown_rx, state_tx).await
            });

            (
                fsm,
                #handle_name {
                    event_tx,
                    shutdown_tx,
                    state_rx,
                },
                #task_name { handle },
            )
        }
    }
}

/// Generate the run implementation with event loop.
fn generate_run_impl(fsm: &FsmStructure) -> TokenStream {
    let context_type = &fsm.context_type;
    let error_type = &fsm.error_type;
    
    // Generate event match arms
    let event_arms = generate_event_match_arms(fsm);
    
    // Check if we have timeout handlers
    let has_timeout_handler = fsm.handlers.iter().any(|h| h.is_timeout_handler);
    let has_state_timeout = fsm.handlers.iter().any(|h| h.state_timeout.is_some());
    
    // Find timeout handler method name
    let timeout_handler_name = fsm.handlers.iter()
        .find(|h| h.is_timeout_handler)
        .map(|h| &h.method.sig.ident);
    
    // Find state timeout duration (simplified - use first one found)
    let timeout_duration = fsm.handlers.iter()
        .find_map(|h| h.state_timeout.as_ref().map(|st| st.duration.value()));
    
    if has_timeout_handler && has_state_timeout && timeout_handler_name.is_some() && timeout_duration.is_some() {
        let handler_name = timeout_handler_name.unwrap();
        let duration_str = timeout_duration.unwrap();
        
        quote! {
            async fn run(
                mut self,
                mut events: tokio::sync::mpsc::Receiver<Event>,
                mut shutdown: tokio::sync::watch::Receiver<Option<tokio_fsm_core::ShutdownMode>>,
                state_tx: tokio::sync::watch::Sender<State>,
            ) -> Result<#context_type, #error_type> {
                use std::time::Duration;
                use std::pin::Pin;
                use tokio::time::{Sleep, Instant};
                
                let duration = tokio_fsm_core::parse_duration(#duration_str)
                    .expect("Invalid timeout duration");
                let mut timeout: Option<Pin<Box<Sleep>>> = None;
                
                loop {
                    tokio::select! {
                        Some(event) = events.recv() => {
                            match (&self.state, event) {
                                #(#event_arms)*
                                (state, event) => {
                                    // Invalid transition - log and ignore
                                    #[cfg(feature = "tracing")]
                                    tracing::warn!("Invalid transition: {:?} with {:?}", state, event);
                                }
                            }
                        }
                        
                        _ = async {
                            if let Some(ref mut t) = timeout {
                                t.as_mut().await
                            } else {
                                std::future::pending().await
                            }
                        }, if timeout.is_some() => {
                            // Timeout occurred
                            let transition = self.#handler_name().await;
                            let new_state = transition.into_state();
                            self.state = new_state;
                            let _ = state_tx.send(self.state);
                            timeout = None; // Clear timeout
                        }
                        
                        Ok(()) = shutdown.changed() => {
                            match *shutdown.borrow() {
                                Some(tokio_fsm_core::ShutdownMode::Graceful) => {
                                    // Process remaining events
                                    while let Ok(event) = events.try_recv() {
                                        match (&self.state, event) {
                                            #(#event_arms)*
                                            _ => {}
                                        }
                                    }
                                    break;
                                }
                                Some(tokio_fsm_core::ShutdownMode::Immediate) => break,
                                None => {}
                            }
                        }
                    }
                }
                Ok(self.context)
            }
        }
    } else {
        // No timeout handling
        quote! {
            async fn run(
                mut self,
                mut events: tokio::sync::mpsc::Receiver<Event>,
                mut shutdown: tokio::sync::watch::Receiver<Option<tokio_fsm_core::ShutdownMode>>,
                state_tx: tokio::sync::watch::Sender<State>,
            ) -> Result<#context_type, #error_type> {
                loop {
                    tokio::select! {
                        Some(event) = events.recv() => {
                            match (&self.state, event) {
                                #(#event_arms)*
                                (state, event) => {
                                    // Invalid transition - log and ignore
                                    #[cfg(feature = "tracing")]
                                    tracing::warn!("Invalid transition: {:?} with {:?}", state, event);
                                }
                            }
                        }
                        
                        Ok(()) = shutdown.changed() => {
                            match *shutdown.borrow() {
                                Some(tokio_fsm_core::ShutdownMode::Graceful) => {
                                    // Process remaining events
                                    while let Ok(event) = events.try_recv() {
                                        match (&self.state, event) {
                                            #(#event_arms)*
                                            _ => {}
                                        }
                                    }
                                    break;
                                }
                                Some(tokio_fsm_core::ShutdownMode::Immediate) => break,
                                None => {}
                            }
                        }
                    }
                }
                Ok(self.context)
            }
        }
    }
}

/// Generate match arms for state/event combinations.
fn generate_event_match_arms(fsm: &FsmStructure) -> Vec<TokenStream> {
    let mut arms = Vec::new();
    
    for handler in &fsm.handlers {
        if let Some(ref event) = handler.event {
            let method_name = &handler.method.sig.ident;
            let event_name = &event.name;
            
            // Check if handler returns Result or just Transition
            let is_result = match &handler.method.sig.output {
                syn::ReturnType::Type(_, ty) => {
                    if let syn::Type::Path(path) = ty.as_ref() {
                        path.path.segments.last()
                            .map(|seg| seg.ident == "Result")
                            .unwrap_or(false)
                    } else {
                        false
                    }
                }
                syn::ReturnType::Default => false,
            };
            
            if let Some(ref _payload_type) = event.payload_type {
                // Event with payload
                if is_result {
                    arms.push(quote! {
                        (State::_, Event::#event_name(payload)) => {
                            match self.#method_name(payload).await {
                                Ok(transition) => {
                                    let new_state = transition.into_state();
                                    self.state = new_state;
                                    let _ = state_tx.send(self.state);
                                }
                                Err(error_transition) => {
                                    let new_state = error_transition.into_state();
                                    self.state = new_state;
                                    let _ = state_tx.send(self.state);
                                }
                            }
                        }
                    });
                } else {
                    arms.push(quote! {
                        (State::_, Event::#event_name(payload)) => {
                            let transition = self.#method_name(payload).await;
                            let new_state = transition.into_state();
                            self.state = new_state;
                            let _ = state_tx.send(self.state);
                        }
                    });
                }
            } else {
                // Event without payload
                if is_result {
                    arms.push(quote! {
                        (State::_, Event::#event_name) => {
                            match self.#method_name().await {
                                Ok(transition) => {
                                    let new_state = transition.into_state();
                                    self.state = new_state;
                                    let _ = state_tx.send(self.state);
                                }
                                Err(error_transition) => {
                                    let new_state = error_transition.into_state();
                                    self.state = new_state;
                                    let _ = state_tx.send(self.state);
                                }
                            }
                        }
                    });
                } else {
                    arms.push(quote! {
                        (State::_, Event::#event_name) => {
                            let transition = self.#method_name().await;
                            let new_state = transition.into_state();
                            self.state = new_state;
                            let _ = state_tx.send(self.state);
                        }
                    });
                }
            }
        }
    }
    
    arms
}


/// Generate the Handle implementation.
fn generate_handle_impl(fsm: &FsmStructure) -> TokenStream {
    let handle_name = format_ident!("{}Handle", fsm.fsm_name);

    quote! {
        impl #handle_name {
            pub async fn send(&self, event: Event) -> Result<(), tokio::sync::mpsc::error::SendError<Event>> {
                self.event_tx.send(event).await
            }

            pub fn try_send(&self, event: Event) -> Result<(), tokio::sync::mpsc::error::TrySendError<Event>> {
                self.event_tx.try_send(event)
            }

            pub async fn shutdown_graceful(self) {
                let _ = self.shutdown_tx.send(Some(tokio_fsm_core::ShutdownMode::Graceful));
            }

            pub async fn shutdown_immediate(self) {
                let _ = self.shutdown_tx.send(Some(tokio_fsm_core::ShutdownMode::Immediate));
            }

            pub fn current_state(&self) -> State {
                *self.state_rx.borrow()
            }

            pub async fn wait_for_state(&mut self, target: State) -> Result<(), tokio::sync::watch::error::RecvError> {
                loop {
                    if *self.state_rx.borrow() == target {
                        return Ok(());
                    }
                    self.state_rx.changed().await?;
                }
            }
        }
    }
}
