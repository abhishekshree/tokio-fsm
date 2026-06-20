//! Shared TokenStream builders and private FSM helper generation.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Error;

use crate::validation::{FsmStructure, HandlerReturnKind};

pub fn render_fsm_private_helpers(fsm: &FsmStructure) -> syn::Result<TokenStream> {
    let fsm_name = &fsm.fsm_name;
    let state_enum_name = fsm.state_enum_ident();
    let event_enum_name = fsm.event_enum_ident();
    let context_type = &fsm.context_type;
    let error_type = &fsm.error_type;
    let initial_state = &fsm.initial_state;
    let apply_arms = build_apply_arms(fsm)?;

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

    Ok(quote! {
        /// Creates an FSM in its initial state.
        pub fn new(context: #context_type) -> Self {
            #fsm_name {
                state: #state_enum_name::#initial_state,
                context,
            }
        }

        /// Returns the current state of the FSM.
        pub fn state(&self) -> #state_enum_name {
            self.state
        }

        /// Returns a shared reference to the FSM context.
        pub fn context(&self) -> &#context_type {
            &self.context
        }

        /// Returns a mutable reference to the FSM context.
        pub fn context_mut(&mut self) -> &mut #context_type {
            &mut self.context
        }

        /// Consumes the FSM and returns its context.
        pub fn into_context(self) -> #context_type {
            self.context
        }

        /// Applies an event directly to this FSM.
        pub async fn apply(&mut self, event: #event_enum_name) -> Result<#state_enum_name, ::tokio_fsm::ApplyError<#event_enum_name, #state_enum_name, #error_type>> {
            match (self.state, event) {
                #(#apply_arms)*
                (state, event) => Err(::tokio_fsm::ApplyError::Unhandled { state, event }),
            }
        }

        fn apply_transition(
            &mut self,
            next: #state_enum_name,
            event_name: &str,
        ) -> #state_enum_name {
            let old_state = self.state;
            self.state = next;
            #tracing_log
            self.state
        }

        fn apply_transition_and_notify(
            &mut self,
            next: #state_enum_name,
            state_tx: &::tokio_fsm::tokio::sync::watch::Sender<#state_enum_name>,
            event_name: &str,
        ) -> #state_enum_name {
            let state = self.apply_transition(next, event_name);
            let _ = state_tx.send(state);
            state
        }
    })
}

fn build_apply_arms(fsm: &FsmStructure) -> syn::Result<Vec<TokenStream>> {
    let mut arms = Vec::new();
    let event_enum = fsm.event_enum_ident();
    let state_enum = fsm.state_enum_ident();

    for handler in &fsm.handlers {
        if let Some(ref event) = handler.event {
            let event_name = event.name();
            let event_name_str = event_name.to_string();
            let method_name = &handler.method.sig.ident;
            let targets = handler.targets.as_ref().ok_or_else(|| {
                Error::new_spanned(
                    &handler.method.sig.ident,
                    "Internal macro error: missing parsed targets for event handler",
                )
            })?;

            let (payload_pattern, payload_call) = if event.payload_type().is_some() {
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
                    let target_state = targets.static_target().ok_or_else(|| {
                        Error::new_spanned(
                            &handler.method.sig.ident,
                            "Internal macro error: expected static target",
                        )
                    })?;
                    let target_state = target_state.as_ref();
                    quote! {
                        self.#method_name #payload_call.await;
                        Ok(self.apply_transition(#state_enum::#target_state, #event_name_str))
                    }
                }
                HandlerReturnKind::ResultUnit => {
                    let target_state = targets.static_target().ok_or_else(|| {
                        Error::new_spanned(
                            &handler.method.sig.ident,
                            "Internal macro error: expected static target",
                        )
                    })?;
                    let target_state = target_state.as_ref();
                    quote! {
                        match self.#method_name #payload_call.await {
                            Ok(()) => Ok(self.apply_transition(#state_enum::#target_state, #event_name_str)),
                            Err(error) => Err(::tokio_fsm::ApplyError::HandlerFailed(error)),
                        }
                    }
                }
                HandlerReturnKind::Transition => {
                    let allowed_targets = targets.dynamic_targets().ok_or_else(|| {
                        Error::new_spanned(
                            &handler.method.sig.ident,
                            "Internal macro error: expected dynamic targets",
                        )
                    })?;
                    let allowed_targets: Vec<_> =
                        allowed_targets.iter().map(|state| state.as_ref()).collect();
                    quote! {
                        let next = self.#method_name #payload_call.await.into_state();
                        match next {
                            #(#state_enum::#allowed_targets)|* => Ok(self.apply_transition(next, #event_name_str)),
                            state => Err(::tokio_fsm::ApplyError::InvalidTransition { state }),
                        }
                    }
                }
                HandlerReturnKind::ResultTransitionError => {
                    let allowed_targets = targets.dynamic_targets().ok_or_else(|| {
                        Error::new_spanned(
                            &handler.method.sig.ident,
                            "Internal macro error: expected dynamic targets",
                        )
                    })?;
                    let allowed_targets: Vec<_> =
                        allowed_targets.iter().map(|state| state.as_ref()).collect();
                    quote! {
                        match self.#method_name #payload_call.await {
                            Ok(transition) => {
                                let next = transition.into_state();
                                match next {
                                    #(#state_enum::#allowed_targets)|* => Ok(self.apply_transition(next, #event_name_str)),
                                    state => Err(::tokio_fsm::ApplyError::InvalidTransition { state }),
                                }
                            }
                            Err(error) => Err(::tokio_fsm::ApplyError::HandlerFailed(error)),
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
