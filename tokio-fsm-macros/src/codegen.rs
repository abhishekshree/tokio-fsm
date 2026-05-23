use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItem, ItemImpl};

use crate::validation::FsmStructure;

pub mod enums;
pub mod impls;
pub mod structs;

/// Main entry point for code generation.
/// Takes the validated FSM structure and the original impl block,
/// generates all types, impls, and the event loop.
///
/// High-level flow:
/// 1. Parse macro input into `FsmStructure`
/// 2. Validate graph and handler semantics
/// 3. Generate enums, structs, and impl blocks
///
/// See `tokio-fsm-macros/CODEGEN.md` for contributor-oriented notes on where to
/// edit the codegen when behavior changes.
pub fn generate(fsm: &FsmStructure, original_impl: &ItemImpl) -> syn::Result<TokenStream> {
    let fsm_name = &fsm.fsm_name;
    let original_methods = &original_impl.items;

    // Generate type definitions
    let state_enum = enums::render_state_enum(fsm);
    let event_enum = enums::render_event_enum(fsm);

    let fsm_struct = structs::render_fsm_struct(fsm);
    let handle_struct = structs::render_handle_struct(fsm);
    let task_struct = structs::render_task_struct(fsm);

    // Generate implementations
    let spawn_impl = impls::render_spawn(fsm);
    let run_impl = impls::render_run(fsm)?;
    let fsm_private_impl = impls::render_fsm_private_helpers(fsm)?;
    let handle_impl = impls::render_handle_impl(fsm);
    let task_impl = impls::render_task_impl(fsm);
    let task_drop = impls::render_task_drop(fsm);

    // Strip macro attributes from original methods, remove associated types
    let cleaned_items: Vec<ImplItem> = original_methods
        .iter()
        .filter_map(|item| match item {
            ImplItem::Fn(method) => {
                let mut method = method.clone();
                method.attrs.retain(|attr| !attr.path().is_ident("on"));
                Some(ImplItem::Fn(method))
            }
            ImplItem::Type(_) => None,
            _ => Some(item.clone()),
        })
        .collect();

    Ok(quote! {
        #state_enum
        #event_enum

        #fsm_struct
        #handle_struct
        #task_struct

        impl #fsm_name {
            #spawn_impl
            #run_impl
            #fsm_private_impl

            #(#cleaned_items)*
        }

        #handle_impl
        #task_impl
        #task_drop
    })
}
