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
use super::graph_analysis::*;
use super::graph_cross::*;
use super::graph_topology::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj534_graph_timeline_flag() {
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
            timeline_graph: true,
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
            Commands::Graph(GraphArgs { timeline_graph, .. }) => assert!(timeline_graph),
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj544_graph_what_if_flag() {
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
            what_if: Some("base-packages".to_string()),
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
            Commands::Graph(GraphArgs { what_if, .. }) => {
                assert_eq!(what_if, Some("base-packages".to_string()))
            }
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj554_graph_blast_radius_flag() {
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
            blast_radius: Some("base-packages".to_string()),
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
            Commands::Graph(GraphArgs { blast_radius, .. }) => {
                assert_eq!(blast_radius, Some("base-packages".to_string()))
            }
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj564_graph_change_impact_flag() {
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
            change_impact: Some("base-packages".to_string()),
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
            Commands::Graph(GraphArgs { change_impact, .. }) => {
                assert_eq!(change_impact, Some("base-packages".to_string()))
            }
            _ => panic!("expected Graph"),
        }
    }


    #[test]
    fn test_fj574_graph_resource_types_flag() {
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
            resource_types: true,
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
            Commands::Graph(GraphArgs { resource_types, .. }) => assert!(resource_types),
            _ => panic!("expected Graph"),
        }
    }

}
