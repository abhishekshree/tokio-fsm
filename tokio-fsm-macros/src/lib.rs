//! Proc macro for generating Tokio async finite state machines.

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemImpl};

mod attrs;
mod codegen;
mod validation;

/// Main proc macro entry point.
///
/// Generates a complete FSM implementation from a trait-like impl block.
#[proc_macro_attribute]
pub fn fsm(args: TokenStream, input: TokenStream) -> TokenStream {
    use syn::parse::{Parse, ParseStream};
    use syn::Token;
    
    struct FsmArgsParser {
        initial: Option<syn::LitStr>,
        channel_size: usize,
    }
    
    impl Parse for FsmArgsParser {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut initial = None;
            let mut channel_size = 100;
            
            if input.is_empty() {
                return Err(syn::Error::new(input.span(), "Missing required 'initial' argument"));
            }
            
            loop {
                let ident: syn::Ident = input.parse()?;
                let _eq: Token![=] = input.parse()?;
                
                if ident == "initial" {
                    let lit: syn::LitStr = input.parse()?;
                    initial = Some(lit);
                } else if ident == "channel_size" {
                    let lit: syn::LitInt = input.parse()?;
                    channel_size = lit.base10_parse()?;
                } else {
                    return Err(syn::Error::new(ident.span(), "Unknown argument"));
                }
                
                if input.is_empty() {
                    break;
                }
                let _comma: Token![,] = input.parse()?;
            }
            
            Ok(FsmArgsParser {
                initial,
                channel_size,
            })
        }
    }
    
    let input_impl = parse_macro_input!(input as ItemImpl);
    
    let parser: FsmArgsParser = match syn::parse(args) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };
    
    let initial = parser.initial.ok_or_else(|| {
        syn::Error::new(proc_macro2::Span::call_site(), "Missing required 'initial' argument")
    }).unwrap();
    
    let fsm_args = attrs::FsmArgs {
        initial,
        channel_size: parser.channel_size,
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

