use std::collections::{HashMap, HashSet};

use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::Dfs,
};
use syn::Ident;

use super::types::FsmStructure;

impl FsmStructure {
    /// Validate the FSM graph for reachability and semantic correctness.
    pub fn validate(&self) -> syn::Result<()> {
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

        self.build_reachability_graph(&mut graph, &nodes)?;
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
                let key = (source.to_string(), event.name().to_string());
                if let Some(existing_handler) = handlers_by_transition.get(&key) {
                    return Err(syn::Error::new_spanned(
                        &handler.method.sig.ident,
                        format!(
                            "Duplicate handler for state '{}' and event '{}': '{}' and '{}'",
                            source,
                            event.name(),
                            existing_handler,
                            handler.method.sig.ident
                        ),
                    ));
                }
                handlers_by_transition.insert(key, &handler.method.sig.ident);
            }
        }

        Ok(())
    }

    fn build_reachability_graph(
        &self,
        graph: &mut DiGraph<&Ident, ()>,
        nodes: &HashMap<&Ident, NodeIndex>,
    ) -> syn::Result<()> {
        for handler in &self.handlers {
            for target in &handler.target_states {
                let target_node = nodes.get(&target.name).ok_or_else(|| {
                    syn::Error::new_spanned(&target.name, "Target state not found")
                })?;

                for source_ident in &handler.source_states {
                    let source_node = nodes.get(source_ident).ok_or_else(|| {
                        syn::Error::new_spanned(source_ident, "Source state not found")
                    })?;
                    graph.add_edge(*source_node, *target_node, ());
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
}
