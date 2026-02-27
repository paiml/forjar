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
use super::commands::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj454_graph_prune_flag() {
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
            prune: Some("web-server".to_string()),
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
        });
        match cmd {
            Commands::Graph(GraphArgs { prune, .. }) => {
                assert_eq!(prune, Some("web-server".to_string()));
            }
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj464_graph_layers_flag() {
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
            layers: true,
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
        });
        match cmd {
            Commands::Graph(GraphArgs { layers, .. }) => assert!(layers),
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj474_graph_critical_resources_flag() {
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
            critical_resources: true,
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
        });
        match cmd {
            Commands::Graph(GraphArgs {
                critical_resources, ..
            }) => assert!(critical_resources),
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj484_graph_weight_flag() {
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
            weight: true,
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
        });
        match cmd {
            Commands::Graph(GraphArgs { weight, .. }) => assert!(weight),
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj494_graph_subgraph_flag() {
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
            subgraph: Some("my-resource".to_string()),
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
        });
        match cmd {
            Commands::Graph(GraphArgs { subgraph, .. }) => {
                assert_eq!(subgraph, Some("my-resource".to_string()))
            }
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj504_graph_impact_radius_flag() {
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
            impact_radius: Some("base-packages".to_string()),
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
        });
        match cmd {
            Commands::Graph(GraphArgs { impact_radius, .. }) => {
                assert_eq!(impact_radius, Some("base-packages".to_string()))
            }
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj514_graph_dependency_matrix_flag() {
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
            dependency_matrix: true,
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
        });
        match cmd {
            Commands::Graph(GraphArgs {
                dependency_matrix, ..
            }) => assert!(dependency_matrix),
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
        });
        match cmd {
            Commands::Graph(GraphArgs { hotspots, .. }) => assert!(hotspots),
            _ => panic!("expected Graph"),
        }
    }

}
