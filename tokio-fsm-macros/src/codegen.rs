use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemImpl;

use crate::validation::FsmStructure;

pub mod enums;
pub mod impls;
pub mod structs;

/// Main entry point for code generation.
/// Takes the validated FSM structure and the original impl block,
/// generates all types, impls, and the event loop.
pub fn generate(fsm: &FsmStructure, original_impl: &ItemImpl) -> TokenStream {
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
    let run_impl = impls::render_run(fsm);
    let handle_impl = impls::render_handle_impl(fsm);
    let task_impl = impls::render_task_impl(fsm);

    // Strip macro attributes from original methods, remove associated types
    let cleaned_items: Vec<syn::ImplItem> = original_methods
        .iter()
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method) => {
                let mut method = method.clone();
                method.attrs.retain(|attr| {
                    !attr.path().is_ident("on")
                        && !attr.path().is_ident("state_timeout")
                        && !attr.path().is_ident("on_timeout")
                });
                Some(syn::ImplItem::Fn(method))
            }
            syn::ImplItem::Type(_) => None,
            _ => Some(item.clone()),
        })
        .collect();

    quote! {
        #state_enum
        #event_enum

        #fsm_struct
        #handle_struct
        #task_struct

        impl #fsm_name {
            #spawn_impl
            #run_impl

            #(#cleaned_items)*
        }

        #handle_impl
        #task_impl
    }
}
