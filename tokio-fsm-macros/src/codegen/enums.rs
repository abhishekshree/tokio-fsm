use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::FsmIr;

pub fn render_state_enum(ir: &FsmIr) -> TokenStream {
    let states: Vec<_> = ir.states.iter().map(|s| &s.name).collect();
    let state_enum_name = &ir.state_enum_ident;

    let state_structs: Vec<_> = ir
        .states
        .iter()
        .map(|s| {
            let name = &s.name;
            quote! {
                #[derive(Debug, Clone, Copy)]
                pub struct #name;
                impl From<#name> for #state_enum_name {
                    fn from(_: #name) -> Self {
                        #state_enum_name::#name
                    }
                }
            }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #state_enum_name {
            #(#states,)*
        }

        #(#state_structs)*
    }
}

pub fn render_event_enum(ir: &FsmIr) -> TokenStream {
    let variants: Vec<TokenStream> = ir
        .events
        .iter()
        .map(|event| {
            let event_name = &event.name;
            if let Some(ref payload_type) = event.payload_type {
                quote! { #event_name(#payload_type), }
            } else {
                quote! { #event_name, }
            }
        })
        .collect();

    let event_enum_name = &ir.event_enum_ident;

    quote! {
        #[derive(Debug, Clone)]
        pub enum #event_enum_name {
            #(#variants)*
        }
    }
}
