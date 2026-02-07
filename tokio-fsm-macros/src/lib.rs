//! Proc macro for generating Tokio async finite state machines.

use darling::FromMeta;
use proc_macro::TokenStream;
use syn::{ItemImpl, parse_macro_input};

mod attrs;
mod codegen;
mod validation;

/// Main proc macro entry point.
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
    // Parse the FSM structure
    let fsm_structure = validation::FsmStructure::parse(args, input.clone())?;

    // Generate the code
    Ok(codegen::generate(&fsm_structure, &input))
}
