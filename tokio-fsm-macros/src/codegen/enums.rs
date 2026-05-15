use proc_macro2::TokenStream;
use quote::quote;

use crate::validation::FsmStructure;

pub fn render_state_enum(fsm: &FsmStructure) -> TokenStream {
    let states: Vec<_> = fsm.states.iter().map(|s| &s.name).collect();
    let state_enum_name = fsm.state_enum_ident();

    let state_structs: Vec<_> = fsm
        .states
        .iter()
        .map(|s| {
            let marker_name = fsm.state_marker_ident(&s.name);
            let state_name = &s.name;
            let enum_name = &state_enum_name;
            quote! {
                #[doc(hidden)]
                #[derive(Debug, Clone, Copy)]
                pub struct #marker_name;
                impl From<#marker_name> for #enum_name {
                    fn from(_: #marker_name) -> Self {
                        #enum_name::#state_name
                    }
                }
            }
        })
        .collect();

    if fsm.serde {
        quote! {
            ::tokio_fsm::__tokio_fsm_serde_derive! {
                #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                pub enum #state_enum_name {
                    #(#states,)*
                }
            }

            #(#state_structs)*
        }
    } else {
        quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum #state_enum_name {
                #(#states,)*
            }

            #(#state_structs)*
        }
    }
}

pub fn render_event_enum(fsm: &FsmStructure) -> TokenStream {
    let variants: Vec<TokenStream> = fsm
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

    let debug_arms: Vec<TokenStream> = fsm
        .events
        .iter()
        .map(|event| {
            let event_name = &event.name;
            let event_name_str = event_name.to_string();
            if event.payload_type.is_some() {
                quote! {
                    Self::#event_name(_) => f.write_str(#event_name_str),
                }
            } else {
                quote! {
                    Self::#event_name => f.write_str(#event_name_str),
                }
            }
        })
        .collect();

    let event_enum_name = fsm.event_enum_ident();

    let event_enum = if fsm.serde {
        quote! {
            ::tokio_fsm::__tokio_fsm_serde_derive! {
                pub enum #event_enum_name {
                    #(#variants)*
                }
            }
        }
    } else {
        quote! {
            pub enum #event_enum_name {
                #(#variants)*
            }
        }
    };

    quote! {
        #event_enum

        impl ::std::fmt::Debug for #event_enum_name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #(#debug_arms)*
                }
            }
        }
    }
}
