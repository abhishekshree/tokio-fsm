use syn::{Error, FnArg, GenericArgument, ImplItem, ItemImpl, PathArguments, ReturnType, Type};

use super::types::{
    EventSpec, FsmStructure, Handler, HandlerReturnKind, State, StateName, TransitionTargets,
};
use crate::attrs;

impl Handler {
    /// Parse a method into a Handler with all semantic fields derived.
    pub fn parse(method: &syn::ImplItemFn) -> syn::Result<Self> {
        if method.sig.asyncness.is_none() {
            return Err(Error::new_spanned(
                &method.sig.ident,
                "FSM handlers must be declared as async fn",
            ));
        }

        let mut event: Option<EventSpec> = None;
        let mut source_states = Vec::new();
        let mut target_names: Option<Vec<StateName>> = None;

        // Parse attributes
        for attr in &method.attrs {
            if attr.path().is_ident("on") {
                let on_attr = attrs::OnAttr::parse(attr)?;
                let payload_type = if method.sig.inputs.len() > 1 {
                    if let FnArg::Typed(pat_type) = &method.sig.inputs[1] {
                        Some((*pat_type.ty).clone())
                    } else {
                        None
                    }
                } else {
                    None
                };
                // Multiple #[on(...)] attributes are allowed for multi-state handlers
                if source_states.iter().any(|state| state == &on_attr.state) {
                    return Err(Error::new_spanned(
                        attr,
                        format!(
                            "Duplicate #[on] source state '{}' on handler '{}'",
                            on_attr.state, method.sig.ident
                        ),
                    ));
                }
                source_states.push(on_attr.state);
                if let Some(ref existing_event) = event {
                    if existing_event.name() != &on_attr.event {
                        return Err(Error::new_spanned(
                            &on_attr.event,
                            format!(
                                "Handler method '{}' handles multiple different events: '{}' and '{}'",
                                method.sig.ident,
                                existing_event.name(),
                                on_attr.event
                            ),
                        ));
                    }
                } else {
                    event = Some(EventSpec::from_parts(on_attr.event, payload_type));
                }
                if let Some(existing_targets) = &target_names {
                    let next_states: Vec<_> =
                        on_attr.next.iter().map(ToString::to_string).collect();
                    let existing_states: Vec<_> = existing_targets
                        .iter()
                        .map(|state| state.as_ref().to_string())
                        .collect();
                    if next_states != existing_states {
                        return Err(Error::new_spanned(
                            attr,
                            "Multiple #[on] attributes on one handler must use the same next states",
                        ));
                    }
                } else {
                    target_names = Some(on_attr.next.into_iter().map(StateName::from).collect());
                }
            } else if attr.path().is_ident("on_timeout") || attr.path().is_ident("state_timeout") {
                return Err(Error::new_spanned(
                    attr,
                    "timeout attributes were removed; model timeout events explicitly",
                ));
            }
        }

        let (targets, return_kind) = if event.is_some() {
            let return_kind = parse_handler_return(&method.sig.output)?;
            let target_names = target_names.ok_or_else(|| {
                Error::new_spanned(
                    &method.sig.ident,
                    "Internal macro error: missing parsed target states for event handler",
                )
            })?;
            let targets = parse_transition_targets(return_kind, target_names, &method.sig.ident)?;
            (Some(targets), Some(return_kind))
        } else {
            (None, None)
        };

        Ok(Self {
            method: method.clone(),
            event,
            targets,
            return_kind,
            source_states,
        })
    }
}

fn parse_transition_targets(
    return_kind: HandlerReturnKind,
    target_names: Vec<StateName>,
    method_name: &syn::Ident,
) -> syn::Result<TransitionTargets> {
    match return_kind {
        HandlerReturnKind::Unit | HandlerReturnKind::ResultUnit => {
            let mut targets = target_names.into_iter();
            let Some(target) = targets.next() else {
                return Err(Error::new_spanned(
                    method_name,
                    "handlers returning () or Result<(), E> must declare exactly one next state",
                ));
            };
            if targets.next().is_some() {
                return Err(Error::new_spanned(
                    method_name,
                    "handlers returning () or Result<(), E> must declare exactly one next state",
                ));
            }
            Ok(TransitionTargets::Static(target))
        }
        HandlerReturnKind::Transition | HandlerReturnKind::ResultTransitionError => {
            Ok(TransitionTargets::Dynamic(target_names))
        }
    }
}

fn parse_handler_return(output: &ReturnType) -> syn::Result<HandlerReturnKind> {
    let return_type = match output {
        ReturnType::Type(_, ty) => ty.as_ref(),
        ReturnType::Default => return Ok(HandlerReturnKind::Unit),
    };

    parse_return_kind(return_type)
}

fn parse_return_kind(ty: &Type) -> syn::Result<HandlerReturnKind> {
    if matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty()) {
        return Ok(HandlerReturnKind::Unit);
    }

    let Type::Path(path) = ty else {
        return Err(Error::new_spanned(
            ty,
            "FSM handlers must return (), Result<(), E>, Transition<State>, or Result<Transition<State>, E>",
        ));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(Error::new_spanned(
            ty,
            "FSM handlers must return (), Result<(), E>, Transition<State>, or Result<Transition<State>, E>",
        ));
    };

    match segment.ident.to_string().as_str() {
        "Transition" => {
            ensure_transition_state(ty)?;
            Ok(HandlerReturnKind::Transition)
        }
        "Result" => parse_result_return(segment, ty),
        _ => Err(Error::new_spanned(
            ty,
            "FSM handlers must return (), Result<(), E>, Transition<State>, or Result<Transition<State>, E>",
        )),
    }
}

fn parse_result_return(segment: &syn::PathSegment, ty: &Type) -> syn::Result<HandlerReturnKind> {
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Err(Error::new_spanned(
            ty,
            "Result handlers must return Result<(), E> or Result<Transition<State>, E>",
        ));
    };

    let type_args: Vec<&Type> = args
        .args
        .iter()
        .filter_map(|arg| match arg {
            GenericArgument::Type(inner_ty) => Some(inner_ty),
            _ => None,
        })
        .collect();

    if type_args.len() != 2 {
        return Err(Error::new_spanned(
            ty,
            "Result handlers must return Result<(), E> or Result<Transition<State>, E>",
        ));
    }

    if matches!(type_args[0], Type::Tuple(tuple) if tuple.elems.is_empty()) {
        return Ok(HandlerReturnKind::ResultUnit);
    }

    ensure_transition_state(type_args[0]).map_err(|_| {
        Error::new_spanned(
            type_args[0],
            "Result handlers must return Result<(), E> or Result<Transition<State>, E>",
        )
    })?;
    Ok(HandlerReturnKind::ResultTransitionError)
}

fn ensure_transition_state(ty: &Type) -> syn::Result<()> {
    let Type::Path(path) = ty else {
        return Err(Error::new_spanned(ty, "Expected Transition<State>"));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(Error::new_spanned(ty, "Expected Transition<State>"));
    };

    if segment.ident != "Transition" {
        return Err(Error::new_spanned(ty, "Expected Transition<State>"));
    }

    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Err(Error::new_spanned(ty, "Expected Transition<State>"));
    };

    let mut type_args = args.args.iter().filter_map(|arg| match arg {
        GenericArgument::Type(inner_ty) => Some(inner_ty),
        _ => None,
    });

    if type_args.next().is_none() {
        return Err(Error::new_spanned(ty, "Expected Transition<State>"));
    };
    if type_args.next().is_some() {
        return Err(Error::new_spanned(ty, "Expected Transition<State>"));
    }
    Ok(())
}

impl FsmStructure {
    /// Parse the impl block and extract the complete FSM structure.
    pub fn parse(args: attrs::FsmArgs, impl_block: &ItemImpl) -> syn::Result<Self> {
        let fsm_name = match &*impl_block.self_ty {
            Type::Path(path) => path
                .path
                .segments
                .last()
                .ok_or_else(|| Error::new_spanned(&impl_block.self_ty, "Expected FSM type name"))?
                .ident
                .clone(),
            _ => {
                return Err(Error::new_spanned(
                    &impl_block.self_ty,
                    "Expected type path for FSM",
                ));
            }
        };

        let initial_state = args.initial;

        // Extract associated types
        let mut context_type = None;
        let mut error_type = None;

        for item in &impl_block.items {
            if let ImplItem::Type(ty) = item {
                if ty.ident == "Context" {
                    context_type = Some(ty.ty.clone());
                } else if ty.ident == "Error" {
                    error_type = Some(ty.ty.clone());
                }
            }
        }

        let context_type = context_type.ok_or_else(|| {
            Error::new_spanned(impl_block, "Missing associated type: type Context = ...")
        })?;
        let error_type = error_type.ok_or_else(|| {
            Error::new_spanned(impl_block, "Missing associated type: type Error = ...")
        })?;

        // Parse methods
        let mut handlers = Vec::new();
        let mut events: Vec<EventSpec> = Vec::new();
        let mut state_names: Vec<syn::Ident> = Vec::new();

        state_names.push(initial_state.clone());

        for item in &impl_block.items {
            if let ImplItem::Fn(method) = item {
                let handler = Handler::parse(method)?;

                // Collect states from declared targets
                if let Some(targets) = &handler.targets {
                    for state in targets.states() {
                        let state = state.as_ref();
                        if !state_names.iter().any(|s| s == state) {
                            state_names.push(state.clone());
                        }
                    }
                }

                // Collect source states
                for state in &handler.source_states {
                    if !state_names.iter().any(|s| s == state) {
                        state_names.push(state.clone());
                    }
                }

                // Collect events and validate payload consistency
                if let Some(ref event) = handler.event {
                    if let Some(existing_event) = events
                        .iter()
                        .find(|existing| existing.name() == event.name())
                    {
                        if existing_event != event {
                            let expected = existing_event.payload_description();
                            let actual = event.payload_description();
                            return Err(Error::new_spanned(
                                event.name(),
                                format!(
                                    "Event '{}' has inconsistent payload types across handlers: expected '{}', found '{}'",
                                    event.name(),
                                    expected,
                                    actual
                                ),
                            ));
                        }
                    } else {
                        events.push(event.clone());
                    }
                }

                handlers.push(handler);
            }
        }

        let states: Vec<State> = state_names.into_iter().map(|name| State { name }).collect();

        let fsm = Self {
            fsm_name,
            initial_state,
            channel_size: args.channel_size,
            context_type,
            error_type,
            states,
            events,
            handlers,
            tracing: args.tracing,
            serde: args.serde,
        };

        fsm.validate()?;

        Ok(fsm)
    }
}
