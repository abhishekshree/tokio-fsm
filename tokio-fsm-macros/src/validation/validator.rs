use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GraphDiagnostic {
    UnreachableState { state: String },
}

/// Build a forward adjacency list from the edge set.
fn build_adjacency(edges: &[(String, String)]) -> HashMap<&str, Vec<&str>> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (source, target) in edges {
        adj.entry(source.as_str()).or_default().push(target.as_str());
    }
    adj
}

/// Compute the set of states reachable from `initial` via the given adjacency
/// list. Zero heap allocations for strings.
fn reachable_from<'a>(
    initial: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
) -> BTreeSet<&'a str> {
    let mut reachable = BTreeSet::new();
    let mut stack = vec![initial];

    while let Some(state) = stack.pop() {
        if !reachable.insert(state) {
            continue;
        }
        if let Some(targets) = adj.get(state) {
            for target in targets {
                stack.push(target);
            }
        }
    }

    reachable
}

/// Validate a state machine graph for reachability.
///
/// Returns a list of `GraphDiagnostic` warnings or errors.
pub(crate) fn validate_graph(
    initial: &str,
    states: &[String],
    edges: &[(String, String)],
) -> Vec<GraphDiagnostic> {
    let mut diagnostics = Vec::new();
    let adj = build_adjacency(edges);
    let reachable = reachable_from(initial, &adj);

    // 1. Check unreachable states
    for state in states {
        if !reachable.contains(state.as_str()) {
            diagnostics.push(GraphDiagnostic::UnreachableState {
                state: state.clone(),
            });
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEBSOCKET_EDGES: &[(&str, &str)] = &[
        ("Disconnected", "Connecting"),
        ("Connecting", "Authenticating"),
        ("Connecting", "Backoff"),
        ("Authenticating", "Subscribing"),
        ("Authenticating", "Backoff"),
        ("Subscribing", "Live"),
        ("Subscribing", "Backoff"),
        ("Live", "Degraded"),
        ("Live", "Reconnecting"),
        ("Live", "Closing"),
        ("Degraded", "Live"),
        ("Degraded", "Reconnecting"),
        ("Reconnecting", "Connecting"),
        ("Reconnecting", "Backoff"),
        ("Backoff", "Connecting"),
        ("Backoff", "Closing"),
        ("Closing", "Closed"),
    ];

    fn websocket_states() -> Vec<String> {
        vec![
            "Disconnected".to_string(),
            "Connecting".to_string(),
            "Authenticating".to_string(),
            "Subscribing".to_string(),
            "Live".to_string(),
            "Degraded".to_string(),
            "Reconnecting".to_string(),
            "Backoff".to_string(),
            "Closing".to_string(),
            "Closed".to_string(),
        ]
    }

    fn to_owned_edges(edges: &[(&str, &str)]) -> Vec<(String, String)> {
        edges
            .iter()
            .map(|(s, t)| (s.to_string(), t.to_string()))
            .collect()
    }

    #[test]
    fn canonical_websocket_graph_has_no_diagnostics() {
        let states = websocket_states();
        let edges = to_owned_edges(WEBSOCKET_EDGES);

        let diags = validate_graph("Disconnected", &states, &edges);
        assert!(diags.is_empty());
    }

    #[derive(Clone, Debug)]
    struct WebSocketSubgraph {
        mask: u32,
    }

    impl quickcheck::Arbitrary for WebSocketSubgraph {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mask = u32::arbitrary(g) & ((1 << WEBSOCKET_EDGES.len()) - 1);
            WebSocketSubgraph { mask }
        }
    }

    quickcheck::quickcheck! {
        fn prop_graph_validation_matches_reference_implementation(subgraph: WebSocketSubgraph) -> bool {
            let active_edges: Vec<(String, String)> = WEBSOCKET_EDGES
                .iter()
                .enumerate()
                .filter(|(idx, _)| (subgraph.mask & (1 << idx)) != 0)
                .map(|(_, &(s, t))| (s.to_string(), t.to_string()))
                .collect();

            let states = websocket_states();

            // 1. Run our optimized graph validator
            let diags = validate_graph("Disconnected", &states, &active_edges);

            // 2. Run simple reference BFS reachability
            let mut ref_reachable = std::collections::HashSet::new();
            let mut stack = vec!["Disconnected"];
            while let Some(curr) = stack.pop() {
                if ref_reachable.insert(curr) {
                    for (src, tgt) in &active_edges {
                        if src == curr {
                            stack.push(tgt);
                        }
                    }
                }
            }

            // 3. Compare unreachable states
            for state in &states {
                let is_unreachable = !ref_reachable.contains(state.as_str());
                let reported_unreachable = diags.iter().any(|d| match d {
                    GraphDiagnostic::UnreachableState { state: s } => s == state,
                });
                if is_unreachable != reported_unreachable {
                    return false;
                }
            }

            true
        }
    }
}
