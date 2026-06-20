use proc_macro2::TokenStream;
use quote::quote;

use crate::validation::FsmStructure;

pub fn render_handle_impl(fsm: &FsmStructure) -> TokenStream {
    let handle_name = fsm.handle_ident();
    let event_enum_name = fsm.event_enum_ident();
    let state_enum_name = fsm.state_enum_ident();
    let command_name = fsm.command_enum_ident();
    let error_type = &fsm.error_type;

    quote! {
        impl #handle_name {
            /// Applies an event and waits until the FSM processes it.
            pub async fn apply(&self, event: #event_enum_name) -> Result<#state_enum_name, ::tokio_fsm::ApplyError<#event_enum_name, #state_enum_name, #error_type>> {
                let (reply, response) = ::tokio_fsm::tokio::sync::oneshot::channel();
                self.event_tx
                    .send(#command_name::Event { event, reply })
                    .await
                    .map_err(|error| match error.0 {
                        #command_name::Event { event, .. } => ::tokio_fsm::ApplyError::Closed(event),
                    })?;
                response.await.map_err(|_| ::tokio_fsm::ApplyError::Interrupted)?
            }

            /// Returns the current state of the FSM.
            pub fn state(&self) -> #state_enum_name {
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

        }
    }
}
