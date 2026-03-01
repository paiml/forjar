//! Tests: Core graph commands.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::graph_core::*;
use super::graph_analysis::*;
use super::graph_cross::*;
use super::graph_topology::*;
use super::commands::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj584_graph_topological_levels_flag() {
        let cmd = Commands::Graph(GraphArgs {
            file: PathBuf::from("forjar.yaml"),
            format: "mermaid".to_string(),
            machine: None,
            group: None,
            affected: None,
            critical_path: false,
            reverse: false,
            depth: None,
            cluster: false,
            orphans: false,
            stats: false,
            json_output: false,
            highlight: None,
            prune: None,
            layers: false,
            critical_resources: false,
            weight: false,
            subgraph: None,
            impact_radius: None,
            dependency_matrix: false,
            hotspots: false,
            timeline_graph: false,
            what_if: None,
            blast_radius: None,
            change_impact: None,
            resource_types: false,
            topological_levels: true,
            execution_order: false,
            security_boundaries: false,
            resource_age: false,
            parallel_groups: false,
            critical_chain: false,
            dependency_depth: false,
            orphan_detection: false,
            cross_machine_deps: false,
            machine_groups: false,
            resource_clusters: false,
            fan_out: false,
            leaf_resources: false,
            reverse_deps: false,
            depth_first: false,
            breadth_first: false,
            subgraph_stats: false,
            dependency_count: false,
            root_resources: false,
            edge_list: false,
            connected_components: false,
            adjacency_matrix: false,
            longest_path: false,
            in_degree: false,
            out_degree: false,
            density: false,
            topological_sort: false,
            critical_path_resources: false,
            sink_resources: false,
            bipartite_check: false,
            strongly_connected: false,
            dependency_matrix_csv: false, resource_weight: false, dependency_depth_per_resource: false, resource_fanin: false, isolated_subgraphs: false, resource_dependency_chain: None, bottleneck_resources: false, critical_dependency_path: false, resource_depth_histogram: false, resource_coupling_score: false, resource_change_frequency: false,
                resource_impact_score: false,
                resource_stability_score: false,
                resource_dependency_fanout: false, resource_dependency_weight: false, resource_dependency_bottleneck: false, resource_type_clustering: false, resource_dependency_cycle_risk: false, resource_impact_radius: false, resource_dependency_health_map: false, resource_change_propagation: false, resource_dependency_depth_analysis: false, resource_dependency_fan_analysis: false, resource_dependency_isolation_score: false, resource_dependency_stability_score: false, resource_dependency_critical_path_length: false, resource_dependency_redundancy_score: false, resource_dependency_centrality_score: false, resource_dependency_bridge_detection: false, resource_dependency_cluster_coefficient: false, resource_dependency_modularity_score: false, resource_dependency_diameter: false, resource_dependency_eccentricity: false, resource_dependency_density: false, resource_dependency_transitivity: false, resource_dependency_fan_out: false, resource_dependency_fan_in: false, resource_dependency_path_count: false, resource_dependency_articulation_points: false, resource_dependency_longest_path: false, resource_dependency_strongly_connected: false, resource_dependency_topological_depth: false, resource_dependency_weak_links: false, resource_dependency_minimum_cut: false, resource_dependency_dominator_tree: false, resource_dependency_resilience_score: false, resource_dependency_pagerank: false, resource_dependency_betweenness_centrality: false, resource_dependency_closure_size: false, resource_dependency_eccentricity_map: false, resource_dependency_diameter_path: false, resource_dependency_bridge_criticality: false, resource_dependency_conditional_subgraph: false, resource_dependency_parallel_groups: false, resource_dependency_execution_cost: false, resource_recipe_expansion_map: false, resource_dependency_critical_chain_path: false, resource_apply_order_simulation: false, resource_provenance_summary: false,
            resource_dependency_risk_score: false,
            resource_dependency_layering: false,
            resource_lifecycle_stage_map: false,
            resource_dependency_age_overlay: false, resource_dependency_health_overlay: false, resource_dependency_width_analysis: false, resource_dependency_critical_path_highlight: false, resource_dependency_bottleneck_detection: false,
                resource_topology_cluster_analysis: false,
                resource_dependency_island_detection: false,
                resource_dependency_depth_histogram_analysis: false,
                resource_dependency_redundancy_analysis: false,
        });
        match cmd {
            Commands::Graph(GraphArgs {
                topological_levels, ..
            }) => assert!(topological_levels),
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj595_graph_execution_order() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n  cfg1:\n    type: file\n    machine: m1\n    path: /tmp/test\n    content: hello\n    depends_on: [pkg1]\n").unwrap();
        let result = cmd_graph_execution_order(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj595_graph_execution_order_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_graph_execution_order(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj604_graph_security_boundaries() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_graph_security_boundaries(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj604_graph_security_boundaries_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  fw1:\n    type: network\n    machine: m1\n    port: 22\n    protocol: tcp\n    action: allow\n").unwrap();
        let result = cmd_graph_security_boundaries(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj614_graph_resource_age() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_graph_resource_age(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj614_graph_resource_age_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_graph_resource_age(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj624_graph_parallel_groups() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n  cfg1:\n    type: file\n    machine: m1\n    path: /tmp/test\n    content: hello\n    depends_on: [pkg1]\n").unwrap();
        let result = cmd_graph_parallel_groups(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj624_graph_parallel_groups_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_graph_parallel_groups(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj634_graph_critical_chain() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n  cfg1:\n    type: file\n    machine: m1\n    path: /tmp/test\n    content: hello\n    depends_on: [pkg1]\n").unwrap();
        let result = cmd_graph_critical_chain(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj634_graph_critical_chain_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_graph_critical_chain(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj644_graph_dependency_depth() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    depends_on: [a]\n").unwrap();
        let result = cmd_graph_dependency_depth(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj644_graph_dependency_depth_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n").unwrap();
        let result = cmd_graph_dependency_depth(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj654_graph_orphan_detection() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  lonely:\n    type: file\n    machine: m\n    path: /tmp/x\n").unwrap();
        let result = cmd_graph_orphan_detection(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj654_graph_orphan_detection_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    depends_on: [a]\n").unwrap();
        let result = cmd_graph_orphan_detection(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj664_graph_cross_machine_deps() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m1\n    path: /tmp/a\n  b:\n    type: file\n    machine: m2\n    path: /tmp/b\n    depends_on: [a]\n").unwrap();
        let result = cmd_graph_cross_machine_deps(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj664_graph_cross_machine_deps_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n").unwrap();
        let result = cmd_graph_cross_machine_deps(&cfg, true);
        assert!(result.is_ok());
    }

}
