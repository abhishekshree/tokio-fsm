use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::Dfs,
};
use syn::Ident;

use super::types::FsmStructure;

impl FsmStructure {
    /// Validate the FSM graph for reachability and semantic correctness.
    pub fn validate(&self) -> syn::Result<()> {
        let has_timeout_handler = self.validate_timeout_contract()?;
        let states_with_timeout = self.identify_states_with_timeout(has_timeout_handler)?;
        self.validate_state_timeout_consistency()?;
        self.validate_unique_event_handlers()?;

        let mut graph = DiGraph::<&Ident, ()>::new();
        let mut nodes = HashMap::new();

        for state in &self.states {
            let node = graph.add_node(&state.name);
            nodes.insert(&state.name, node);
        }

        let initial_node = nodes.get(&self.initial_state).ok_or_else(|| {
            syn::Error::new_spanned(&self.initial_state, "Initial state not found")
        })?;

        self.build_reachability_graph(&mut graph, &nodes, &states_with_timeout)?;
        self.check_reachability(&graph, initial_node, &nodes)?;

        Ok(())
    }

    fn validate_unique_event_handlers(&self) -> syn::Result<()> {
        let mut handlers_by_transition: HashMap<(String, String), &syn::Ident> = HashMap::new();

        for handler in &self.handlers {
            let Some(event) = &handler.event else {
                continue;
            };

            for source in &handler.source_states {
                let key = (source.to_string(), event.name.to_string());
                if let Some(existing_handler) = handlers_by_transition.get(&key) {
                    return Err(syn::Error::new_spanned(
                        &handler.method.sig.ident,
                        format!(
                            "Duplicate handler for state '{}' and event '{}': '{}' and '{}'",
                            source, event.name, existing_handler, handler.method.sig.ident
                        ),
                    ));
                }
                handlers_by_transition.insert(key, &handler.method.sig.ident);
            }
        }

        Ok(())
    }

    fn validate_timeout_contract(&self) -> syn::Result<bool> {
        let timeout_handlers: Vec<_> = self
            .handlers
            .iter()
            .filter(|h| h.is_timeout_handler)
            .collect();
        if timeout_handlers.len() > 1 {
            return Err(syn::Error::new_spanned(
                &timeout_handlers[1].method.sig.ident,
                "Multiple #[on_timeout] handlers are not allowed",
            ));
        }
        Ok(!timeout_handlers.is_empty())
    }

    fn identify_states_with_timeout(
        &self,
        has_timeout_handler: bool,
    ) -> syn::Result<HashSet<&Ident>> {
        let mut states_with_timeout = HashSet::new();
        for handler in &self.handlers {
            if handler.timeout.is_some() {
                if !has_timeout_handler {
                    return Err(syn::Error::new_spanned(
                        &handler.method.sig.ident,
                        "#[state_timeout] requires an #[on_timeout] handler",
                    ));
                }
                for target in &handler.return_states {
                    states_with_timeout.insert(&target.name);
                }
            }
        }
        Ok(states_with_timeout)
    }

    fn build_reachability_graph(
        &self,
        graph: &mut DiGraph<&Ident, ()>,
        nodes: &HashMap<&Ident, NodeIndex>,
        states_with_timeout: &HashSet<&Ident>,
    ) -> syn::Result<()> {
        for handler in &self.handlers {
            for target in &handler.return_states {
                let target_node = nodes.get(&target.name).ok_or_else(|| {
                    syn::Error::new_spanned(&target.name, "Target state not found")
                })?;

                if handler.is_timeout_handler {
                    for &state_name in states_with_timeout {
                        let source_node = nodes.get(state_name).ok_or_else(|| {
                            syn::Error::new_spanned(
                                state_name,
                                "Internal error: timeout source state not found",
                            )
                        })?;
                        graph.add_edge(*source_node, *target_node, ());
                    }
                } else {
                    for source_ident in &handler.source_states {
                        let source_node = nodes.get(source_ident).ok_or_else(|| {
                            syn::Error::new_spanned(source_ident, "Source state not found")
                        })?;
                        graph.add_edge(*source_node, *target_node, ());
                    }
                }
            }
        }
        Ok(())
    }

    fn check_reachability(
        &self,
        graph: &DiGraph<&Ident, ()>,
        initial_node: &NodeIndex,
        nodes: &HashMap<&Ident, NodeIndex>,
    ) -> syn::Result<()> {
        let mut dfs = Dfs::new(graph, *initial_node);
        let mut reachable = HashSet::new();
        while let Some(node) = dfs.next(graph) {
            reachable.insert(node);
        }

        for (&name, &node) in nodes {
            if !reachable.contains(&node) {
                return Err(syn::Error::new_spanned(
                    name,
                    format!(
                        "State '{}' is unreachable from initial state '{}'",
                        name, self.initial_state
                    ),
                ));
            }
        }
        Ok(())
    }

    fn validate_state_timeout_consistency(&self) -> syn::Result<()> {
        let mut timeout_by_state: HashMap<String, (Duration, &syn::Ident)> = HashMap::new();

        for handler in &self.handlers {
            let Some(timeout) = handler.timeout else {
                continue;
            };

            for target in &handler.return_states {
                let state_name = target.name.to_string();
                if let Some((existing_timeout, existing_handler)) =
                    timeout_by_state.get(&state_name)
                {
                    if *existing_timeout != timeout {
                        return Err(syn::Error::new_spanned(
                            &handler.method.sig.ident,
                            format!(
                                "State '{}' has conflicting timeout durations declared by '{}' and '{}'",
                                target.name, existing_handler, handler.method.sig.ident
                            ),
                        ));
                    }
                } else {
                    timeout_by_state.insert(state_name, (timeout, &handler.method.sig.ident));
                }
            }
        }

        Ok(())
    }
}
