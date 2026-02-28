//! Tests: Core graph commands (overflow from core_1/core_2).

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::graph_core::*;
use super::commands::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj444_graph_highlight_flag() {
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
            highlight: Some("web-server".to_string()),
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
            topological_levels: false,
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
            dependency_matrix_csv: false,
            resource_weight: false,
            dependency_depth_per_resource: false, resource_fanin: false, isolated_subgraphs: false, resource_dependency_chain: None, bottleneck_resources: false, critical_dependency_path: false, resource_depth_histogram: false, resource_coupling_score: false, resource_change_frequency: false,
                resource_impact_score: false,
                resource_stability_score: false,
                resource_dependency_fanout: false, resource_dependency_weight: false, resource_dependency_bottleneck: false, resource_type_clustering: false, resource_dependency_cycle_risk: false, resource_impact_radius: false, resource_dependency_health_map: false, resource_change_propagation: false, resource_dependency_depth_analysis: false, resource_dependency_fan_analysis: false, resource_dependency_isolation_score: false, resource_dependency_stability_score: false, resource_dependency_critical_path_length: false, resource_dependency_redundancy_score: false, resource_dependency_centrality_score: false, resource_dependency_bridge_detection: false, resource_dependency_cluster_coefficient: false, resource_dependency_modularity_score: false, resource_dependency_diameter: false, resource_dependency_eccentricity: false, resource_dependency_density: false, resource_dependency_transitivity: false, resource_dependency_fan_out: false, resource_dependency_fan_in: false, resource_dependency_path_count: false, resource_dependency_articulation_points: false, resource_dependency_longest_path: false, resource_dependency_strongly_connected: false, resource_dependency_topological_depth: false, resource_dependency_weak_links: false, resource_dependency_minimum_cut: false, resource_dependency_dominator_tree: false, resource_dependency_resilience_score: false, resource_dependency_pagerank: false, resource_dependency_betweenness_centrality: false, resource_dependency_closure_size: false, resource_dependency_eccentricity_map: false, resource_dependency_diameter_path: false, resource_dependency_bridge_criticality: false, resource_dependency_conditional_subgraph: false, resource_dependency_parallel_groups: false, resource_dependency_execution_cost: false, resource_recipe_expansion_map: false, resource_dependency_critical_chain_path: false, resource_apply_order_simulation: false, resource_provenance_summary: false,
            resource_dependency_risk_score: false,
            resource_dependency_layering: false,
            resource_lifecycle_stage_map: false,
            resource_dependency_age_overlay: false, resource_dependency_health_overlay: false, resource_dependency_width_analysis: false,
        });
        match cmd {
            Commands::Graph(GraphArgs { highlight, .. }) => {
                assert_eq!(highlight, Some("web-server".to_string()));
            }
            _ => panic!("expected Graph"),
        }
    }

    #[test]
    fn test_fj524_graph_hotspots_flag() {
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
            hotspots: true,
            timeline_graph: false,
            what_if: None,
            blast_radius: None,
            change_impact: None,
            resource_types: false,
            topological_levels: false,
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
            dependency_matrix_csv: false,
            resource_weight: false,
            dependency_depth_per_resource: false, resource_fanin: false, isolated_subgraphs: false, resource_dependency_chain: None, bottleneck_resources: false, critical_dependency_path: false, resource_depth_histogram: false, resource_coupling_score: false, resource_change_frequency: false,
                resource_impact_score: false,
                resource_stability_score: false,
                resource_dependency_fanout: false, resource_dependency_weight: false, resource_dependency_bottleneck: false, resource_type_clustering: false, resource_dependency_cycle_risk: false, resource_impact_radius: false, resource_dependency_health_map: false, resource_change_propagation: false, resource_dependency_depth_analysis: false, resource_dependency_fan_analysis: false, resource_dependency_isolation_score: false, resource_dependency_stability_score: false, resource_dependency_critical_path_length: false, resource_dependency_redundancy_score: false, resource_dependency_centrality_score: false, resource_dependency_bridge_detection: false, resource_dependency_cluster_coefficient: false, resource_dependency_modularity_score: false, resource_dependency_diameter: false, resource_dependency_eccentricity: false, resource_dependency_density: false, resource_dependency_transitivity: false, resource_dependency_fan_out: false, resource_dependency_fan_in: false, resource_dependency_path_count: false, resource_dependency_articulation_points: false, resource_dependency_longest_path: false, resource_dependency_strongly_connected: false, resource_dependency_topological_depth: false, resource_dependency_weak_links: false, resource_dependency_minimum_cut: false, resource_dependency_dominator_tree: false, resource_dependency_resilience_score: false, resource_dependency_pagerank: false, resource_dependency_betweenness_centrality: false, resource_dependency_closure_size: false, resource_dependency_eccentricity_map: false, resource_dependency_diameter_path: false, resource_dependency_bridge_criticality: false, resource_dependency_conditional_subgraph: false, resource_dependency_parallel_groups: false, resource_dependency_execution_cost: false, resource_recipe_expansion_map: false, resource_dependency_critical_chain_path: false, resource_apply_order_simulation: false, resource_provenance_summary: false,
            resource_dependency_risk_score: false,
            resource_dependency_layering: false,
            resource_lifecycle_stage_map: false,
            resource_dependency_age_overlay: false, resource_dependency_health_overlay: false, resource_dependency_width_analysis: false,
        });
        match cmd {
            Commands::Graph(GraphArgs { hotspots, .. }) => assert!(hotspots),
            _ => panic!("expected Graph"),
        }
    }
}
