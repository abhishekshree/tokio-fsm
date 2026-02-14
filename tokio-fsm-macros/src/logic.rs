use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::FsmIr;

/// Builds the match arms for the main event loop.
/// This involves complex logic matching events to handlers, handling payloads,
/// dealing with Results, and resetting timeouts.
pub fn build_event_arms(ir: &FsmIr) -> Vec<TokenStream> {
    let mut arms = Vec::new();
    let event_enum = &ir.event_enum_ident;

    for handler in &ir.handlers {
        if let Some(ref event_name) = handler.event_variant {
            let method_name = &handler.method_name;

            // 1. Timeout Reset Logic
            // 1. Timeout Reset Logic
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

            // 2. Payload Handling
            let (payload_pattern, payload_call) = if handler.has_payload {
                (quote! { (payload) }, quote! { (payload) })
            } else {
                (quote! {}, quote! { () })
            };

            // 3. Result vs Direct Transition
            let arm_inner = if handler.is_async_result {
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

            arms.push(quote! {
                (_, #event_enum::#event_name #payload_pattern) => {
                    #arm_inner
                }
            });
        }
    }

    arms
}

/// Builds the timeout handler block for the run loop.
pub fn build_timeout_handler(ir: &FsmIr) -> TokenStream {
    if let Some(handler) = ir.handlers.iter().find(|h| h.is_timeout_handler) {
        let name = &handler.method_name;
        quote! {
            let transition = self.#name().await;
            self.state = transition.into_state().into();
            let _ = state_tx.send(self.state);
            // Note: We don't verify if the timeout handler *returns* a state with a timeout.
            // For now, we assume it doesn't set a new timeout implicitly unless we add logic here.
            // Usually timeout handler -> Failed/Idle, which don't have timeouts.
        }
    } else {
        quote! {}
    }
}
