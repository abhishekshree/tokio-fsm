//! Rendering for the generated event loop.
//!
//! The run loop owns:
//! - event dispatch
//! - timeout dispatch
//! - cancellation races
//! - tracing instrumentation at the outer loop boundary

use proc_macro2::TokenStream;
use quote::quote;
use syn::Error;

use super::helpers::{render_cancellable_call, render_transition_dispatch};
use crate::validation::FsmStructure;

pub fn render_run(fsm: &FsmStructure) -> syn::Result<TokenStream> {
    let event_enum_name = fsm.event_enum_ident();
    let state_enum_name = fsm.state_enum_ident();
    let context_type = &fsm.context_type;
    let error_type = &fsm.error_type;
    let fsm_name_str = fsm.fsm_name.to_string();

    let event_arms = build_event_arms(fsm)?;
    let event_name_arms = build_event_name_arms(fsm);
    let timeout_logic = build_timeout_handler(fsm)?;

    let tracing_span = if fsm.tracing {
        quote! {
            let span = ::tokio_fsm::tracing::info_span!(
                "fsm",
                name = #fsm_name_str,
                fsm_id = self.name.as_deref().unwrap_or("unnamed")
            );
        }
    } else {
        quote! {}
    };

    let run_loop_await = if fsm.tracing {
        quote! {
            use ::tokio_fsm::tracing::Instrument as _;
            run_loop.instrument(span).await
        }
    } else {
        quote! {
            run_loop.await
        }
    };

    let tracing_cancellation = if fsm.tracing {
        quote! {
            ::tokio_fsm::tracing::info!("FSM received external cancellation");
        }
    } else {
        quote! {}
    };

    let unmatched_arm = if fsm.tracing {
        quote! {
            (state, event) => {
                let event_name = match event {
                    #(#event_name_arms)*
                };
                ::tokio_fsm::tracing::warn!(state = ?state, event = event_name, "Event dropped: No handler for this state");
            }
        }
    } else {
        quote! {
            (_, _) => {}
        }
    };

    Ok(quote! {
        async fn run(
            mut self,
            mut events: ::tokio_fsm::tokio::sync::mpsc::Receiver<#event_enum_name>,
            token: ::tokio_fsm::tokio_util::sync::CancellationToken,
            state_tx: ::tokio_fsm::tokio::sync::watch::Sender<#state_enum_name>,
        ) -> Result<#context_type, #error_type> {
            #tracing_span

            let run_loop = async move {
                let sleep = ::tokio_fsm::tokio::time::sleep(::tokio_fsm::tokio::time::Duration::from_secs(0));
                ::tokio_fsm::tokio::pin!(sleep);
                self.reset_timeout_for_current_state(sleep.as_mut());

                loop {
                    ::tokio_fsm::tokio::select! {
                        _ = &mut sleep => {
                            #timeout_logic
                        }
                        _ = token.cancelled() => {
                            #tracing_cancellation
                            return Ok(self.context);
                        }
                        event = events.recv() => {
                            let Some(event) = event else { break };
                            match (self.state, event) {
                                #(#event_arms)*
                                #unmatched_arm
                            }
                        }
                    }
                }

                Ok(self.context)
            };

            #run_loop_await
        }
    })
}

fn build_event_name_arms(fsm: &FsmStructure) -> Vec<TokenStream> {
    let event_enum = fsm.event_enum_ident();

    fsm.events
        .iter()
        .map(|event| {
            let event_name = &event.name;
            let event_name_str = event_name.to_string();
            if event.payload_type.is_some() {
                quote! {
                    #event_enum::#event_name(_) => #event_name_str,
                }
            } else {
                quote! {
                    #event_enum::#event_name => #event_name_str,
                }
            }
        })
        .collect()
}

fn build_event_arms(fsm: &FsmStructure) -> syn::Result<Vec<TokenStream>> {
    let mut arms = Vec::new();
    let event_enum = fsm.event_enum_ident();
    let state_enum = fsm.state_enum_ident();

    for handler in &fsm.handlers {
        if let Some(ref event) = handler.event {
            let event_name = &event.name;
            let event_name_str = event_name.to_string();
            let method_name = &handler.method.sig.ident;

            let (payload_pattern, payload_call) = if handler.has_payload {
                (quote! { (payload) }, quote! { (payload) })
            } else {
                (quote! {}, quote! { () })
            };

            let handler_call = render_cancellable_call(quote! {
                self.#method_name #payload_call
            });

            let return_kind = handler.return_kind.ok_or_else(|| {
                Error::new_spanned(
                    &handler.method.sig.ident,
                    "Internal macro error: missing parsed return kind for event handler",
                )
            })?;
            let arm_inner = render_transition_dispatch(
                return_kind,
                handler_call,
                quote! { Some(#event_name_str) },
            );

            for source_state in &handler.source_states {
                arms.push(quote! {
                    (#state_enum::#source_state, #event_enum::#event_name #payload_pattern) => {
                        #arm_inner
                    }
                });
            }
        }
    }

    Ok(arms)
}

fn build_timeout_handler(fsm: &FsmStructure) -> syn::Result<TokenStream> {
    if let Some(handler) = fsm.handlers.iter().find(|h| h.is_timeout_handler) {
        let method_name = &handler.method.sig.ident;
        let timeout_call = render_cancellable_call(quote! {
            self.#method_name()
        });

        let return_kind = handler.return_kind.ok_or_else(|| {
            Error::new_spanned(
                &handler.method.sig.ident,
                "Internal macro error: missing parsed return kind for timeout handler",
            )
        })?;

        Ok(render_transition_dispatch(
            return_kind,
            timeout_call,
            quote! { None },
        ))
    } else {
        Ok(quote! {})
    }
}
