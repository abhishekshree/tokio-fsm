use proc_macro2::TokenStream;
use quote::quote;

use crate::validation::FsmStructure;

pub fn render_handle_impl(fsm: &FsmStructure) -> TokenStream {
    let handle_name = fsm.handle_ident();
    let event_enum_name = fsm.event_enum_ident();
    let state_enum_name = fsm.state_enum_ident();
    let can_handle_expr = build_can_handle_expr(fsm);

    quote! {
        impl #handle_name {
            /// Sends an event only if the current observed state can handle it.
            pub async fn send(&self, event: #event_enum_name) -> Result<(), ::tokio_fsm::SendError<#event_enum_name, #state_enum_name>> {
                let state = self.current_state();
                if !Self::can_handle_event(state, &event) {
                    return Err(::tokio_fsm::SendError::Unhandled { state, event });
                }

                self.event_tx
                    .send(event)
                    .await
                    .map_err(|error| ::tokio_fsm::SendError::Closed(error.0))
            }

            /// Enqueues an event without checking whether the current state can handle it.
            pub async fn enqueue(&self, event: #event_enum_name) -> Result<(), ::tokio_fsm::tokio::sync::mpsc::error::SendError<#event_enum_name>> {
                self.event_tx.send(event).await
            }

            /// Attempts to enqueue an event without awaiting capacity.
            pub fn try_enqueue(&self, event: #event_enum_name) -> Result<(), ::tokio_fsm::tokio::sync::mpsc::error::TrySendError<#event_enum_name>> {
                self.event_tx.try_send(event)
            }

            /// Returns the current state of the FSM.
            pub fn current_state(&self) -> #state_enum_name {
                *self.state_rx.borrow()
            }

            /// Waits for the FSM to reach the specified state.
            pub async fn wait_for_state(&self, target: #state_enum_name) -> Result<(), ::tokio_fsm::tokio::sync::watch::error::RecvError> {
                let mut rx = self.state_rx.clone();
                while *rx.borrow_and_update() != target {
                    rx.changed().await?;
                }
                Ok(())
            }

            /// Requests cooperative shutdown of the FSM.
            ///
            /// This cancels the FSM's child token. If the FSM was spawned with
            /// `spawn_with_token`, the parent token remains untouched.
            pub fn shutdown(&self) {
                self.token.cancel();
            }

            /// Returns the cancellation token owned by this handle.
            pub fn token(&self) -> &::tokio_fsm::tokio_util::sync::CancellationToken {
                &self.token
            }

            /// Returns the name of the FSM instance, if provided.
            pub fn name(&self) -> Option<&str> {
                self.name.as_deref()
            }

            fn can_handle_event(state: #state_enum_name, event: &#event_enum_name) -> bool {
                #can_handle_expr
            }
        }
    }
}

fn build_can_handle_expr(fsm: &FsmStructure) -> TokenStream {
    let event_enum_name = fsm.event_enum_ident();
    let state_enum_name = fsm.state_enum_ident();
    let mut arms = Vec::new();

    for handler in &fsm.handlers {
        let Some(event) = &handler.event else {
            continue;
        };

        let event_name = &event.name;
        let event_pattern = if event.payload_type.is_some() {
            quote! { #event_enum_name::#event_name(_) }
        } else {
            quote! { #event_enum_name::#event_name }
        };

        for source in &handler.source_states {
            arms.push(quote! {
                (#state_enum_name::#source, #event_pattern)
            });
        }
    }

    if arms.is_empty() {
        quote! {
            let _ = (state, event);
            false
        }
    } else {
        quote! {
            matches!(
                (state, event),
                #(#arms)|*
            )
        }
    }
}
