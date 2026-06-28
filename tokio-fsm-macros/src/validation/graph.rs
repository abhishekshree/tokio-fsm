use std::collections::{BTreeSet, HashMap};

use syn::Ident;

use super::types::FsmStructure;
use super::validator::{validate_graph, GraphDiagnostic};

impl FsmStructure {
    /// Validate the FSM graph for reachability and semantic correctness.
    pub fn validate(&self) -> syn::Result<()> {
        let state_spans = self.state_spans();
        if !state_spans.contains_key(&self.initial_state.to_string()) {
            return Err(syn::Error::new_spanned(
                &self.initial_state,
                "Initial state not found",
            ));
        }

        self.check_duplicate_handlers()?;

        let states: Vec<String> = self.states.iter().map(|s| s.name.to_string()).collect();
        let edges = self.to_edges(&state_spans)?;
        let diagnostics = validate_graph(
            &self.initial_state.to_string(),
            &states,
            &edges,
        );

        let mut errors: Option<syn::Error> = None;

        for diagnostic in diagnostics {
            let error = self.graph_diagnostic_to_error(diagnostic, &state_spans);
            if let Some(errors) = &mut errors {
                errors.combine(error);
            } else {
                errors = Some(error);
            }
        }

        if let Some(error) = errors {
            return Err(error);
        }

        Ok(())
    }

    fn check_duplicate_handlers(&self) -> syn::Result<()> {
        let mut seen = BTreeSet::new();

        for handler in &self.handlers {
            let Some(event) = &handler.event else {
                continue;
            };
            for source in &handler.source_states {
                let key = (source, event.name());
                #[allow(clippy::collapsible_if)]
                if !seen.insert(key) {
                    let (existing_handler, duplicate_handler) = self
                        .duplicate_handlers_for(source, event.name())
                        .expect("duplicate handler must exist");

                    return Err(syn::Error::new_spanned(
                        duplicate_handler,
                        format!(
                            "Duplicate handler for state '{}' and event '{}': '{}' and '{}'",
                            source,
                            event.name(),
                            existing_handler,
                            duplicate_handler
                        ),
                    ));
                }
            }
        }

        Ok(())
    }

    fn state_spans(&self) -> HashMap<String, Ident> {
        self.states
            .iter()
            .map(|state| (state.name.to_string(), state.name.clone()))
            .collect()
    }

    fn to_edges(
        &self,
        state_spans: &HashMap<String, Ident>,
    ) -> syn::Result<Vec<(String, String)>> {
        let mut edges = Vec::new();

        for handler in &self.handlers {
            let Some(targets) = &handler.targets else {
                continue;
            };

            let target_states = targets.states();

            for target in target_states {
                let target = target.as_ref();
                if !state_spans.contains_key(&target.to_string()) {
                    return Err(syn::Error::new_spanned(target, "Target state not found"));
                }

                for source_ident in &handler.source_states {
                    if !state_spans.contains_key(&source_ident.to_string()) {
                        return Err(syn::Error::new_spanned(
                            source_ident,
                            "Source state not found",
                        ));
                    }

                    edges.push((source_ident.to_string(), target.to_string()));
                }
            }
        }

        Ok(edges)
    }

    fn graph_diagnostic_to_error(
        &self,
        diagnostic: GraphDiagnostic,
        state_spans: &HashMap<String, Ident>,
    ) -> syn::Error {
        match diagnostic {
            GraphDiagnostic::UnreachableState { state } => {
                let span = state_spans.get(&state).unwrap_or(&self.initial_state);
                syn::Error::new_spanned(
                    span,
                    format!(
                        "State '{}' is unreachable from initial state '{}'",
                        state, self.initial_state
                    ),
                )
            }
        }
    }

    fn duplicate_handlers_for(
        &self,
        state: &Ident,
        event: &Ident,
    ) -> Option<(&Ident, &Ident)> {
        let mut first = None;

        for handler in &self.handlers {
            let Some(handler_event) = &handler.event else {
                continue;
            };
            if handler_event.name() != event {
                continue;
            }
            if !handler
                .source_states
                .iter()
                .any(|source| source == state)
            {
                continue;
            }

            let handler_name = &handler.method.sig.ident;
            if let Some(existing_handler) = first {
                return Some((existing_handler, handler_name));
            }
            first = Some(handler_name);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;
    use crate::attrs::FsmArgs;

    #[test]
    fn test_valid_fsm_passes_validation() {
        let args = FsmArgs {
            initial: parse_str("Idle").unwrap(),
            channel_size: 100,
            tracing: false,
            serde: false,
        };
        let input: syn::ItemImpl = parse_str(r#"
            impl MyFsm {
                type Context = ();
                type Error = ();

                #[on(state = Idle, event = Start, next = Running)]
                async fn on_start(&mut self) {}

                #[on(state = Running, event = Stop, next = Idle)]
                async fn on_stop(&mut self) {}
            }
        "#).unwrap();

        let fsm = FsmStructure::parse(args, &input).unwrap();
        assert!(fsm.validate().is_ok());
    }

    #[test]
    fn test_duplicate_handlers_fails_validation() {
        let args = FsmArgs {
            initial: parse_str("Idle").unwrap(),
            channel_size: 100,
            tracing: false,
            serde: false,
        };
        let input: syn::ItemImpl = parse_str(r#"
            impl MyFsm {
                type Context = ();
                type Error = ();

                #[on(state = Idle, event = Start, next = Running)]
                async fn on_start(&mut self) {}

                #[on(state = Idle, event = Start, next = Running)]
                async fn on_start_dup(&mut self) {}
            }
        "#).unwrap();

        let result = FsmStructure::parse(args, &input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Duplicate handler"));
    }

    #[test]
    fn test_unreachable_state_fails_validation() {
        let args = FsmArgs {
            initial: parse_str("Idle").unwrap(),
            channel_size: 100,
            tracing: false,
            serde: false,
        };
        let input: syn::ItemImpl = parse_str(r#"
            impl MyFsm {
                type Context = ();
                type Error = ();

                #[on(state = Idle, event = Start, next = Running)]
                async fn on_start(&mut self) {}

                #[on(state = Unreachable, event = Stop, next = Idle)]
                async fn on_stop(&mut self) {}
            }
        "#).unwrap();

        let result = FsmStructure::parse(args, &input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unreachable"));
    }
}
