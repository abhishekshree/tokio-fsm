//! Rendering for the generated event loop.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Error;

use crate::validation::{FsmStructure, HandlerReturnKind};

pub fn render_run(fsm: &FsmStructure) -> syn::Result<TokenStream> {
    let command_enum_name = fsm.command_enum_ident();
    let state_enum_name = fsm.state_enum_ident();
    let context_type = &fsm.context_type;
    let error_type = &fsm.error_type;
    let fsm_name_str = fsm.fsm_name.to_string();

    let event_arms = build_event_arms(fsm)?;

    let tracing_span = if fsm.tracing {
        quote! {
            let span = ::tokio_fsm::tracing::info_span!("fsm", name = #fsm_name_str);
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

    Ok(quote! {
        async fn run(
            mut self,
            mut events: ::tokio_fsm::tokio::sync::mpsc::Receiver<#command_enum_name>,
            token: ::tokio_fsm::tokio_util::sync::CancellationToken,
            state_tx: ::tokio_fsm::tokio::sync::watch::Sender<#state_enum_name>,
        ) -> Result<#context_type, #error_type> {
            #tracing_span

            let run_loop = async move {
                loop {
                    ::tokio_fsm::tokio::select! {
                        _ = token.cancelled() => {
                            #tracing_cancellation
                            return Ok(self.context);
                        }
                        command = events.recv() => {
                            let Some(command) = command else { break };
                            let #command_enum_name::Event { event, reply } = command;
                            match (self.state, event) {
                                #(#event_arms)*
                                (state, event) => {
                                    let _ = reply.send(Err(::tokio_fsm::ApplyError::Unhandled { state, event }));
                                }
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

fn build_event_arms(fsm: &FsmStructure) -> syn::Result<Vec<TokenStream>> {
    let mut arms = Vec::new();
    let event_enum = fsm.event_enum_ident();
    let state_enum = fsm.state_enum_ident();

    for handler in &fsm.handlers {
        if let Some(ref event) = handler.event {
            let event_name = &event.name;
            let event_name_str = event_name.to_string();
            let method_name = &handler.method.sig.ident;
            let target_state = handler.target_states.first().map(|state| &state.name);
            let allowed_targets: Vec<_> = handler
                .target_states
                .iter()
                .map(|state| &state.name)
                .collect();

            let (payload_pattern, payload_call) = if handler.has_payload {
                (quote! { (payload) }, quote! { (payload) })
            } else {
                (quote! {}, quote! { () })
            };

            let return_kind = handler.return_kind.ok_or_else(|| {
                Error::new_spanned(
                    &handler.method.sig.ident,
                    "Internal macro error: missing parsed return kind for event handler",
                )
            })?;

            let arm_inner = match return_kind {
                HandlerReturnKind::Unit => {
                    let target_state = target_state.ok_or_else(|| {
                        Error::new_spanned(
                            &handler.method.sig.ident,
                            "Internal macro error: missing target state",
                        )
                    })?;
                    quote! {
                        ::tokio_fsm::tokio::select! {
                            _ = token.cancelled() => {
                                let _ = reply.send(Err(::tokio_fsm::ApplyError::Interrupted));
                                return Ok(self.context);
                                }
                            result = self.#method_name #payload_call => result,
                        };
                        let state = self.apply_transition_and_notify(#state_enum::#target_state, &state_tx, #event_name_str);
                        let _ = reply.send(Ok(state));
                    }
                }
                HandlerReturnKind::ResultUnit => {
                    let target_state = target_state.ok_or_else(|| {
                        Error::new_spanned(
                            &handler.method.sig.ident,
                            "Internal macro error: missing target state",
                        )
                    })?;
                    quote! {
                        let result = ::tokio_fsm::tokio::select! {
                            _ = token.cancelled() => {
                                let _ = reply.send(Err(::tokio_fsm::ApplyError::Interrupted));
                                return Ok(self.context);
                            }
                            result = self.#method_name #payload_call => result,
                        };
                        match result {
                            Ok(()) => {
                                let state = self.apply_transition_and_notify(#state_enum::#target_state, &state_tx, #event_name_str);
                                let _ = reply.send(Ok(state));
                            }
                            Err(error) => {
                                let _ = reply.send(Err(::tokio_fsm::ApplyError::HandlerFailed));
                                return Err(error);
                            }
                        }
                    }
                }
                HandlerReturnKind::Transition => {
                    quote! {
                        let transition = ::tokio_fsm::tokio::select! {
                            _ = token.cancelled() => {
                                let _ = reply.send(Err(::tokio_fsm::ApplyError::Interrupted));
                                return Ok(self.context);
                            }
                            result = self.#method_name #payload_call => result,
                        };
                        let next = transition.into_state();
                        match next {
                            #(#state_enum::#allowed_targets)|* => {
                                let state = self.apply_transition_and_notify(next, &state_tx, #event_name_str);
                                let _ = reply.send(Ok(state));
                            }
                            state => {
                                let _ = reply.send(Err(::tokio_fsm::ApplyError::InvalidTransition { state }));
                            }
                        }
                    }
                }
                HandlerReturnKind::ResultTransitionError => {
                    quote! {
                        let result = ::tokio_fsm::tokio::select! {
                            _ = token.cancelled() => {
                                let _ = reply.send(Err(::tokio_fsm::ApplyError::Interrupted));
                                return Ok(self.context);
                            }
                            result = self.#method_name #payload_call => result,
                        };
                        match result {
                            Ok(transition) => {
                                let next = transition.into_state();
                                match next {
                                    #(#state_enum::#allowed_targets)|* => {
                                        let state = self.apply_transition_and_notify(next, &state_tx, #event_name_str);
                                        let _ = reply.send(Ok(state));
                                    }
                                    state => {
                                        let _ = reply.send(Err(::tokio_fsm::ApplyError::InvalidTransition { state }));
                                    }
                                }
                            }
                            Err(error) => {
                                let _ = reply.send(Err(::tokio_fsm::ApplyError::HandlerFailed));
                                return Err(error);
                            }
                        }
                    }
                }
            };

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
