//! Graph command dispatch — routes graph sub-flags to handlers.
use super::graph_advanced::*;
use super::graph_analysis::*;
use super::graph_core::*;
use super::graph_cross::*;
use super::graph_export::*;
use super::graph_extended::*;
use super::graph_impact::*;
use super::graph_paths::*;
use super::graph_scoring::*;
use super::graph_topology::*;
use super::graph_visualization::*;
#[allow(unused_imports)]
use crate::core::types;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub(super) fn try_traversal(
    file: &Path,
    json: bool,
    depth_first: bool,
    reverse_deps: bool,
    leaf_resources: bool,
    fan_out: bool,
    resource_clusters: bool,
    machine_groups: bool,
    cross_machine_deps: bool,
    orphan_detection: bool,
) -> Option<Result<(), String>> {
    if depth_first {
        return Some(cmd_graph_depth_first(file, json));
    }
    if reverse_deps {
        return Some(cmd_graph_reverse_deps(file, json));
    }
    if leaf_resources {
        return Some(cmd_graph_leaf_resources(file, json));
    }
    if fan_out {
        return Some(cmd_graph_fan_out(file, json));
    }
    if resource_clusters {
        return Some(cmd_graph_resource_clusters(file, json));
    }
    if machine_groups {
        return Some(cmd_graph_machine_groups(file, json));
    }
    if cross_machine_deps {
        return Some(cmd_graph_cross_machine_deps(file, json));
    }
    if orphan_detection {
        return Some(cmd_graph_orphan_detection(file, json));
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_topology(
    file: &Path,
    json: bool,
    dependency_depth: bool,
    critical_chain: bool,
    parallel_groups: bool,
    resource_age: bool,
    security_boundaries: bool,
    execution_order: bool,
    topological_levels: bool,
    resource_types: bool,
) -> Option<Result<(), String>> {
    if dependency_depth {
        return Some(cmd_graph_dependency_depth(file, json));
    }
    if critical_chain {
        return Some(cmd_graph_critical_chain(file, json));
    }
    if parallel_groups {
        return Some(cmd_graph_parallel_groups(file, json));
    }
    if resource_age {
        return Some(cmd_graph_resource_age(file, json));
    }
    if security_boundaries {
        return Some(cmd_graph_security_boundaries(file, json));
    }
    if execution_order {
        return Some(cmd_graph_execution_order(file, json));
    }
    if topological_levels {
        return Some(cmd_graph_topological_levels(file, json));
    }
    if resource_types {
        return Some(cmd_graph_resource_types(file, json));
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_impact(
    file: &Path,
    format: &str,
    json: bool,
    change_impact: &Option<String>,
    blast_radius: &Option<String>,
    what_if: &Option<String>,
    timeline_graph: bool,
    hotspots: bool,
    dependency_matrix: bool,
    impact_radius: &Option<String>,
    subgraph: &Option<String>,
    weight: bool,
) -> Option<Result<(), String>> {
    if let Some(ref r) = change_impact {
        return Some(cmd_graph_change_impact(file, r, json));
    }
    if let Some(ref r) = blast_radius {
        return Some(cmd_graph_blast_radius(file, r, json));
    }
    if let Some(ref r) = what_if {
        return Some(cmd_graph_what_if(file, r));
    }
    if timeline_graph {
        return Some(cmd_graph_timeline(file));
    }
    if hotspots {
        return Some(cmd_graph_hotspots(file));
    }
    if dependency_matrix {
        return Some(cmd_graph_dependency_matrix(file, json));
    }
    if let Some(ref r) = impact_radius {
        return Some(cmd_graph_impact_radius(file, r));
    }
    if let Some(ref r) = subgraph {
        return Some(cmd_graph_subgraph(file, format, r));
    }
    if weight {
        return Some(cmd_graph_weight(file, format));
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_visualization(
    file: &Path,
    format: &str,
    json: bool,
    critical_resources: bool,
    layers: bool,
    prune: &Option<String>,
    highlight: &Option<String>,
    stats: bool,
    orphans: bool,
    cluster: bool,
    reverse: bool,
    critical_path: bool,
    affected: &Option<String>,
    depth: Option<usize>,
) -> Option<Result<(), String>> {
    if critical_resources {
        return Some(cmd_graph_critical_resources(file));
    }
    if layers {
        return Some(cmd_graph_layers(file));
    }
    if let Some(ref r) = prune {
        return Some(cmd_graph_prune(file, format, r));
    }
    if let Some(ref r) = highlight {
        return Some(cmd_graph_highlight(file, format, r));
    }
    if json {
        return Some(cmd_graph_json(file));
    }
    if stats {
        return Some(cmd_graph_stats(file));
    }
    if orphans {
        return Some(cmd_graph_orphans(file));
    }
    if cluster {
        return Some(cmd_graph_cluster(file, format));
    }
    if reverse {
        return Some(cmd_graph_reverse(file));
    }
    if critical_path {
        return Some(cmd_graph_critical_path(file));
    }
    if let Some(ref r) = affected {
        return Some(cmd_graph_affected(file, r));
    }
    if let Some(d) = depth {
        return Some(cmd_graph_depth(file, format, d));
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_graph_paths(
    file: &Path,
    json: bool,
    resource_dependency_chain: &Option<String>,
    bottleneck_resources: bool,
    critical_dependency_path: bool,
    resource_depth_histogram: bool,
    resource_coupling_score: bool,
    resource_change_frequency: bool,
    resource_impact_score: bool,
    resource_stability_score: bool,
    resource_dependency_fanout: bool,
    resource_dependency_weight: bool,
) -> Option<Result<(), String>> {
    if let Some(ref target) = resource_dependency_chain {
        return Some(cmd_graph_resource_dependency_chain(file, target, json));
    }
    if bottleneck_resources {
        return Some(cmd_graph_bottleneck_resources(file, json));
    }
    if critical_dependency_path {
        return Some(cmd_graph_critical_dependency_path(file, json));
    }
    if resource_depth_histogram {
        return Some(cmd_graph_resource_depth_histogram(file, json));
    }
    if resource_coupling_score {
        return Some(cmd_graph_resource_coupling_score(file, json));
    }
    if resource_change_frequency {
        return Some(cmd_graph_resource_change_frequency(file, json));
    }
    if resource_impact_score {
        return Some(cmd_graph_resource_impact_score(file, json));
    }
    if resource_stability_score {
        return Some(cmd_graph_resource_stability_score(file, json));
    }
    if resource_dependency_fanout {
        return Some(cmd_graph_resource_dependency_fanout(file, json));
    }
    if resource_dependency_weight {
        return Some(cmd_graph_resource_dependency_weight(file, json));
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_graph_analysis(
    file: &Path,
    json: bool,
    resource_weight: bool,
    dependency_depth_per_resource: bool,
    resource_fanin: bool,
    isolated_subgraphs: bool,
    dependency_matrix_csv: bool,
    strongly_connected: bool,
    bipartite_check: bool,
    sink_resources: bool,
) -> Option<Result<(), String>> {
    if resource_weight {
        return Some(cmd_graph_resource_weight(file, json));
    }
    if dependency_depth_per_resource {
        return Some(cmd_graph_dependency_depth_per_resource(file, json));
    }
    if resource_fanin {
        return Some(cmd_graph_resource_fanin(file, json));
    }
    if isolated_subgraphs {
        return Some(cmd_graph_isolated_subgraphs(file, json));
    }
    if dependency_matrix_csv {
        return Some(cmd_graph_dependency_matrix_csv(file, json));
    }
    if strongly_connected {
        return Some(cmd_graph_strongly_connected(file, json));
    }
    if bipartite_check {
        return Some(cmd_graph_bipartite_check(file, json));
    }
    if sink_resources {
        return Some(cmd_graph_sink_resources(file, json));
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_graph_export_a(
    file: &Path,
    json: bool,
    breadth_first: bool,
    subgraph_stats: bool,
    graph_dependency_count: bool,
    root_resources: bool,
    edge_list: bool,
    connected_components: bool,
    adjacency_matrix: bool,
) -> Option<Result<(), String>> {
    if breadth_first {
        return Some(cmd_graph_breadth_first(file, json));
    }
    if subgraph_stats {
        return Some(cmd_graph_subgraph_stats(file, json));
    }
    if graph_dependency_count {
        return Some(cmd_graph_dependency_count(file, json));
    }
    if root_resources {
        return Some(cmd_graph_root_resources(file, json));
    }
    if edge_list {
        return Some(cmd_graph_edge_list(file, json));
    }
    if connected_components {
        return Some(cmd_graph_connected_components(file, json));
    }
    if adjacency_matrix {
        return Some(cmd_graph_adjacency_matrix(file, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_graph_export_b(
    file: &Path,
    json: bool,
    longest_path: bool,
    in_degree: bool,
    out_degree: bool,
    density: bool,
    topological_sort: bool,
    critical_path_resources: bool,
) -> Option<Result<(), String>> {
    if longest_path {
        return Some(cmd_graph_longest_path(file, json));
    }
    if in_degree {
        return Some(cmd_graph_in_degree(file, json));
    }
    if out_degree {
        return Some(cmd_graph_out_degree(file, json));
    }
    if density {
        return Some(cmd_graph_density(file, json));
    }
    if topological_sort {
        return Some(cmd_graph_topological_sort(file, json));
    }
    if critical_path_resources {
        return Some(cmd_graph_critical_path_resources(file, json));
    }
    None
}

pub(super) use super::dispatch_graph_b::*;
pub(super) use super::dispatch_graph_c::*;
