use super::dag::build_execution_order;
use super::tests_helpers::dag_config;
use proptest::prelude::*;

proptest! {
    /// FALSIFY-DAG-001: Topological ordering — every dependency appears before its dependent.
    #[test]
    fn falsify_dag_001_topo_ordering(
        n in 2..6usize,
        edge_seed in prop::collection::vec((0..5usize, 0..5usize), 0..4),
    ) {
        let names: Vec<String> = (0..n).map(|i| format!("r{i}")).collect();
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        // Build valid DAG edges (only forward edges to avoid cycles)
        let edges: Vec<(&str, &str)> = edge_seed.iter()
            .filter_map(|&(a, b)| {
                let a = a % n;
                let b = b % n;
                if a < b { Some((name_refs[a], name_refs[b])) } else { None }
            })
            .collect();

        let config = dag_config(&name_refs, &edges);
        let order = build_execution_order(&config).unwrap();

        // Verify: for every edge (a -> b), a appears before b
        for (dep, dependent) in &edges {
            let pos_dep = order.iter().position(|x| x == dep).unwrap();
            let pos_dependent = order.iter().position(|x| x == dependent).unwrap();
            prop_assert!(pos_dep < pos_dependent,
                "dependency {} (pos {}) must appear before {} (pos {})",
                dep, pos_dep, dependent, pos_dependent);
        }
    }

    /// FALSIFY-DAG-002: Cycle detection returns Err.
    #[test]
    fn falsify_dag_002_cycle_detection(n in 2..5usize) {
        // Create a cycle: r0 -> r1 -> ... -> r(n-1) -> r0
        let names: Vec<String> = (0..n).map(|i| format!("r{i}")).collect();
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let edges: Vec<(&str, &str)> = (0..n)
            .map(|i| (name_refs[i], name_refs[(i + 1) % n]))
            .collect();

        let config = dag_config(&name_refs, &edges);
        let result = build_execution_order(&config);
        prop_assert!(result.is_err(), "cycle must be detected");
        prop_assert!(result.unwrap_err().contains("cycle"), "error must mention 'cycle'");
    }

}

proptest! {
    /// FALSIFY-DAG-003: Deterministic output — same graph always produces same order.
    #[test]
    fn falsify_dag_003_determinism(
        n in 2..6usize,
        edge_seed in prop::collection::vec((0..5usize, 0..5usize), 0..4),
    ) {
        let names: Vec<String> = (0..n).map(|i| format!("r{i}")).collect();
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let edges: Vec<(&str, &str)> = edge_seed.iter()
            .filter_map(|&(a, b)| {
                let a = a % n;
                let b = b % n;
                if a < b { Some((name_refs[a], name_refs[b])) } else { None }
            })
            .collect();

        let config = dag_config(&name_refs, &edges);
        let order1 = build_execution_order(&config).unwrap();
        let order2 = build_execution_order(&config).unwrap();
        prop_assert_eq!(order1, order2, "build_execution_order must be deterministic");
    }
}
