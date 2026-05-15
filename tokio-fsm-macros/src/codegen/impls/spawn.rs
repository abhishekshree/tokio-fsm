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
        /// Spawns an FSM with a fresh cancellation token.
        #[must_use = "FSM task must be retained or it will be aborted immediately"]
        pub fn spawn(context: #context_type) -> (#handle_name, #task_name) {
            Self::spawn_with_token(context, ::tokio_fsm::tokio_util::sync::CancellationToken::new())
        }

        /// Spawns an FSM linked to a parent cancellation token.
        ///
        /// The generated handle owns a child token, so `handle.shutdown()`
        /// stops the FSM without cancelling the parent token.
        #[must_use = "FSM task must be retained or it will be aborted immediately"]
        pub fn spawn_with_token(context: #context_type, token: ::tokio_fsm::tokio_util::sync::CancellationToken) -> (#handle_name, #task_name) {
            let (event_tx, event_rx) = ::tokio_fsm::tokio::sync::mpsc::channel(#channel_size);
            let (state_tx, state_rx) = ::tokio_fsm::tokio::sync::watch::channel(#state_enum_name::#initial_state);

            let fsm = #fsm_name {
                state: #state_enum_name::#initial_state,
                context,
            };

            let child_token = token.child_token();
            let handle_token = child_token.clone();
            let handle = ::tokio_fsm::tokio::spawn(fsm.run(event_rx, child_token, state_tx));

            (
                #handle_name {
                    event_tx,
                    state_rx,
                    token: handle_token,
                },
                #task_name { handle },
            )
        }
    }
}
