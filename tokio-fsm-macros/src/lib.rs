#![warn(missing_docs)]
//! Proc macro for generating Tokio async finite state machines.

use darling::FromMeta;
use proc_macro::TokenStream;
use syn::{ItemImpl, parse_macro_input};

mod attrs;
mod codegen;
mod validation;

/// Generates an asynchronous Finite State Machine (FSM) from an `impl` block.
///
/// This attribute transforms a standard Rust `impl` block into a compile-time
/// validated state machine driven by a background Tokio task.
///
/// # Arguments
///
/// * `initial = StateName`: (Required) The name of the starting state.
/// * `channel_size = usize`: (Optional) The capacity of the internal event
///   queue (default: 100).
/// * `tracing = true`: (Optional) Enables tracing instrumentation in the
///   generated runtime.
/// * `serde = true`: (Optional) Derives serde support for generated state and
///   event enums when the `tokio-fsm` crate enables its `serde` feature.
///
/// # Generated Types
///
/// The macro generates several types based on the name of the `impl` block
/// (e.g., `WorkerFsm`):
///
/// * `WorkerFsmState`: An enum containing all discovered states.
/// * `WorkerFsmEvent`: An enum containing all discovered events and their data
///   payloads.
/// * `WorkerFsmHandle`: A cloneable handle used to interact with the FSM (send
///   events, query state).
/// * `WorkerFsmTask`: A [`Future`](std::future::Future) that must be awaited to
///   run the FSM. Resolves to `Result<Context, TaskError<E>>`.
///
/// # Handlers & Attributes
///
/// Within the `impl` block, use the following attributes on `async fn` methods:
///
/// * `#[on(state = S, event = E, next = T)]`: Maps a handler to a specific
///   state, event trigger, and target state.
/// * `#[on(state = S, event = E, next = [A, B])]`: Declares dynamic target
///   states for handlers returning `Transition<StateEnum>`.
///
/// More than one handler for the same `(state, event)` pair is rejected at
/// compile time.
///
/// Event handlers may return:
///
/// * `()`
/// * `Result<(), E>`
/// * `Transition<StateEnum>`
/// * `Result<Transition<StateEnum>, E>`
///
/// # Examples
///
/// ```rust,ignore
/// use tokio_fsm::fsm;
///
/// pub struct MyContext;
///
/// #[fsm(initial = Idle)]
/// impl MyFsm {
///     type Context = MyContext;
///     type Error = std::convert::Infallible;
///
///     #[on(state = Idle, event = Start, next = Running)]
///     async fn on_start(&mut self) {
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn fsm(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match darling::ast::NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    let input_impl = parse_macro_input!(input as ItemImpl);

    let fsm_args = match attrs::FsmArgs::from_list(&attr_args) {
        Ok(args) => args,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    match generate_fsm(fsm_args, input_impl) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn generate_fsm(args: attrs::FsmArgs, input: ItemImpl) -> syn::Result<proc_macro2::TokenStream> {
    // 1. Parse + Validate
    let fsm = validation::FsmStructure::parse(args, &input)?;

    // 2. Generate code
    codegen::generate(&fsm, &input)
}
