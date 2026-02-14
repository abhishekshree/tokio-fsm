//! Code generation for FSM implementation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ImplItem;

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

    // Keep the original methods but strip FSM-specific attributes
    let original_methods: Vec<_> = original_impl
        .items
        .iter()
        .filter_map(|item| {
            if let ImplItem::Fn(method) = item {
                let mut stripped_method = method.clone();
                stripped_method.attrs.retain(|attr| {
                    !attr.path().is_ident("event")
                        && !attr.path().is_ident("state_timeout")
                        && !attr.path().is_ident("on_timeout")
                });
                Some(quote! {
                    #stripped_method
                })
            } else {
                None
            }
        })
        .collect();

    let fsm_name = &fsm.fsm_name;

    quote! {
        #state_enum
        #event_enum
        #fsm_struct
        #handle_struct
        #task_struct

        impl #fsm_name {
            #spawn_impl
            #run_impl

            #(#original_methods)*
        }

        #handle_impl
    }
}

fn generate_state_enum(fsm: &FsmStructure) -> TokenStream {
    let states: Vec<_> = fsm.states.iter().map(|s| &s.name).collect();
    let state_enum_name = format_ident!("{}State", fsm.fsm_name);

    let state_structs: Vec<_> = fsm
        .states
        .iter()
        .map(|s| {
            let name = &s.name;
            quote! {
                #[derive(Debug, Clone, Copy)]
                pub struct #name;
                impl From<#name> for #state_enum_name {
                    fn from(_: #name) -> Self {
                        #state_enum_name::#name
                    }
                }
            }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #state_enum_name {
            #(#states,)*
        }

        #(#state_structs)*
    }
}

fn generate_event_enum(fsm: &FsmStructure) -> TokenStream {
    let variants: Vec<TokenStream> = fsm
        .events
        .iter()
        .map(|event| {
            let event_name = &event.name;
            if let Some(ref payload_type) = event.payload_type {
                quote! { #event_name(#payload_type), }
            } else {
                quote! { #event_name, }
            }
        })
        .collect();

    let event_enum_name = format_ident!("{}Event", fsm.fsm_name);

    quote! {
        #[derive(Debug, Clone)]
        pub enum #event_enum_name {
            #(#variants)*
        }
    }
}

fn generate_fsm_struct(fsm: &FsmStructure) -> TokenStream {
    let fsm_name = &fsm.fsm_name;
    let context_type = &fsm.context_type;
    let state_enum_name = format_ident!("{}State", fsm.fsm_name);

    quote! {
        pub struct #fsm_name {
            state: #state_enum_name,
            context: #context_type,
        }
    }
}

fn generate_handle_struct(fsm: &FsmStructure) -> TokenStream {
    let handle_name = format_ident!("{}Handle", fsm.fsm_name);
    let event_enum_name = format_ident!("{}Event", fsm.fsm_name);
    let state_enum_name = format_ident!("{}State", fsm.fsm_name);

    quote! {
        #[derive(Clone)]
        pub struct #handle_name {
            event_tx: tokio::sync::mpsc::Sender<#event_enum_name>,
            shutdown_tx: tokio::sync::watch::Sender<Option<tokio_fsm_core::ShutdownMode>>,
            state_rx: tokio::sync::watch::Receiver<#state_enum_name>,
        }
    }
}

fn generate_task_struct(fsm: &FsmStructure) -> TokenStream {
    let task_name = format_ident!("{}Task", fsm.fsm_name);
    let context_type = &fsm.context_type;
    let error_type = &fsm.error_type;

    quote! {
        #[must_use = "FSM task must be awaited or it will abort"]
        pub struct #task_name {
            handle: tokio::task::JoinHandle<Result<#context_type, #error_type>>,
        }

        impl std::future::Future for #task_name {
            type Output = Result<#context_type, #error_type>;
            fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                use std::future::Future;
                std::pin::Pin::new(&mut self.handle).poll(cx).map(|res| {
                    res.map_err(|e| panic!("FSM task panicked: {:?}", e)).and_then(|r| r)
                })
            }
        }
    }
}

fn generate_spawn_impl(fsm: &FsmStructure) -> TokenStream {
    let fsm_name = &fsm.fsm_name;
    let handle_name = format_ident!("{}Handle", fsm_name);
    let task_name = format_ident!("{}Task", fsm_name);
    let state_enum_name = format_ident!("{}State", fsm_name);
    let context_type = &fsm.context_type;
    let initial_state = &fsm.initial_state;
    let channel_size = fsm.channel_size;

    quote! {
        pub fn spawn(context: #context_type) -> (#handle_name, #task_name) {
            let (event_tx, event_rx) = tokio::sync::mpsc::channel(#channel_size);
            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(None);
            let (state_tx, state_rx) = tokio::sync::watch::channel(#state_enum_name::#initial_state);

            let fsm = #fsm_name {
                state: #state_enum_name::#initial_state,
                context,
            };

            let join_handle = tokio::task::spawn(async move {
                fsm.run(event_rx, shutdown_rx, state_tx).await
            });

            (
                #handle_name {
                    event_tx,
                    shutdown_tx,
                    state_rx,
                },
                #task_name { handle: join_handle },
            )
        }
    }
}

fn generate_run_impl(fsm: &FsmStructure) -> TokenStream {
    let context_type = &fsm.context_type;
    let error_type = &fsm.error_type;
    let event_enum_name = format_ident!("{}Event", fsm.fsm_name);
    let state_enum_name = format_ident!("{}State", fsm.fsm_name);
    let event_arms = generate_event_match_arms(fsm);

    let timeout_handler = fsm.handlers.iter().find(|h| h.is_timeout_handler);
    let timeout_handler_call = if let Some(h) = timeout_handler {
        let name = &h.method.sig.ident;
        quote! {
            let transition = self.#name().await;
            self.state = transition.into_state().into();
            let _ = state_tx.send(self.state);
        }
    } else {
        quote! { /* No timeout handler */ }
    };

    quote! {
        async fn run(
            mut self,
            mut events: tokio::sync::mpsc::Receiver<#event_enum_name>,
            mut shutdown: tokio::sync::watch::Receiver<Option<tokio_fsm_core::ShutdownMode>>,
            state_tx: tokio::sync::watch::Sender<#state_enum_name>,
        ) -> Result<#context_type, #error_type> {
            use std::pin::Pin;
            use tokio::time::Sleep;

            let mut timeout: Option<Pin<Box<Sleep>>> = None;

            loop {
                tokio::select! {
                    event = events.recv() => {
                        let Some(event) = event else { break };
                        match (&self.state, event) {
                            #(#event_arms)*
                            _ => {}
                        }
                    }

                    _ = async {
                        if let Some(ref mut t) = timeout {
                            t.as_mut().await
                        } else {
                            std::future::pending().await
                        }
                    }, if timeout.is_some() => {
                        #timeout_handler_call
                        timeout = None;
                    }

                    res = shutdown.changed() => {
                        if res.is_err() { break; }
                        let mode = *shutdown.borrow();
                        match mode {
                            Some(tokio_fsm_core::ShutdownMode::Graceful) => {
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

fn generate_event_match_arms(fsm: &FsmStructure) -> Vec<TokenStream> {
    let mut arms = Vec::new();

    for handler in &fsm.handlers {
        if let Some(ref event) = handler.event {
            let method_name = &handler.method.sig.ident;
            let event_name = &event.name;
            let event_enum_name = format_ident!("{}Event", fsm.fsm_name);

            let is_result = match &handler.method.sig.output {
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

            let timeout_reset = if let Some(ref st) = handler.state_timeout {
                let duration_str = st.duration.value();
                let duration = match humantime::parse_duration(&duration_str) {
                    Ok(d) => {
                        let secs = d.as_secs();
                        let nanos = d.subsec_nanos();
                        quote! { std::time::Duration::new(#secs, #nanos) }
                    }
                    Err(_) => quote! { std::time::Duration::from_secs(0) }, // Should be validated earlier or handled
                };

                quote! {
                    timeout = Some(Box::pin(tokio::time::sleep(#duration)));
                }
            } else {
                quote! { timeout = None; }
            };

            let arm_inner = if is_result {
                let payload_call = if event.payload_type.is_some() {
                    quote! { (payload) }
                } else {
                    quote! { () }
                };
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
                            timeout = None;
                        }
                    }
                }
            } else {
                let payload_call = if event.payload_type.is_some() {
                    quote! { (payload) }
                } else {
                    quote! { () }
                };
                quote! {
                    let transition = self.#method_name #payload_call .await;
                    self.state = transition.into_state().into();
                    let _ = state_tx.send(self.state);
                    #timeout_reset
                }
            };

            let payload_pattern = if event.payload_type.is_some() {
                quote! { (payload) }
            } else {
                quote! {}
            };

            arms.push(quote! {
                (_, #event_enum_name::#event_name #payload_pattern) => {
                    #arm_inner
                }
            });
        }
    }

    arms
}

fn generate_handle_impl(fsm: &FsmStructure) -> TokenStream {
    let handle_name = format_ident!("{}Handle", fsm.fsm_name);
    let event_enum_name = format_ident!("{}Event", fsm.fsm_name);
    let state_enum_name = format_ident!("{}State", fsm.fsm_name);

    quote! {
        impl #handle_name {
            pub async fn send(&self, event: #event_enum_name) -> Result<(), tokio::sync::mpsc::error::SendError<#event_enum_name>> {
                self.event_tx.send(event).await
            }

            pub fn try_send(&self, event: #event_enum_name) -> Result<(), tokio::sync::mpsc::error::TrySendError<#event_enum_name>> {
                self.event_tx.try_send(event)
            }

            pub fn shutdown_graceful(self) {
                let _ = self.shutdown_tx.send(Some(tokio_fsm_core::ShutdownMode::Graceful));
            }

            pub fn shutdown_immediate(self) {
                let _ = self.shutdown_tx.send(Some(tokio_fsm_core::ShutdownMode::Immediate));
            }

            pub fn current_state(&self) -> #state_enum_name {
                *self.state_rx.borrow()
            }

            pub async fn wait_for_state(&mut self, target: #state_enum_name) -> Result<(), tokio::sync::watch::error::RecvError> {
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
