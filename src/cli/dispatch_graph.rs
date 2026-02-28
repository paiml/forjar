//! Graph command dispatch — routes graph sub-flags to handlers.

#[allow(unused_imports)]
use crate::core::types;
use std::path::Path;
use super::commands::*;
use super::graph_core::*;
use super::graph_topology::*;
use super::graph_analysis::*;
use super::graph_cross::*;
use super::graph_extended::*;
use super::graph_visualization::*;
use super::graph_impact::*;
use super::graph_export::*;
use super::graph_advanced::*;


/// Dispatch traversal flags (depth_first through critical_chain).
fn try_traversal(
    file: &Path, json: bool,
    depth_first: bool, reverse_deps: bool, leaf_resources: bool,
    fan_out: bool, resource_clusters: bool, machine_groups: bool,
    cross_machine_deps: bool, orphan_detection: bool,
) -> Option<Result<(), String>> {
    if depth_first { return Some(cmd_graph_depth_first(file, json)); }
    if reverse_deps { return Some(cmd_graph_reverse_deps(file, json)); }
    if leaf_resources { return Some(cmd_graph_leaf_resources(file, json)); }
    if fan_out { return Some(cmd_graph_fan_out(file, json)); }
    if resource_clusters { return Some(cmd_graph_resource_clusters(file, json)); }
    if machine_groups { return Some(cmd_graph_machine_groups(file, json)); }
    if cross_machine_deps { return Some(cmd_graph_cross_machine_deps(file, json)); }
    if orphan_detection { return Some(cmd_graph_orphan_detection(file, json)); }
    None
}

/// Dispatch topology flags (dependency_depth through resource_types).
fn try_topology(
    file: &Path, json: bool,
    dependency_depth: bool, critical_chain: bool, parallel_groups: bool,
    resource_age: bool, security_boundaries: bool, execution_order: bool,
    topological_levels: bool, resource_types: bool,
) -> Option<Result<(), String>> {
    if dependency_depth { return Some(cmd_graph_dependency_depth(file, json)); }
    if critical_chain { return Some(cmd_graph_critical_chain(file, json)); }
    if parallel_groups { return Some(cmd_graph_parallel_groups(file, json)); }
    if resource_age { return Some(cmd_graph_resource_age(file, json)); }
    if security_boundaries { return Some(cmd_graph_security_boundaries(file, json)); }
    if execution_order { return Some(cmd_graph_execution_order(file, json)); }
    if topological_levels { return Some(cmd_graph_topological_levels(file, json)); }
    if resource_types { return Some(cmd_graph_resource_types(file, json)); }
    None
}

/// Dispatch impact/what-if analysis flags.
fn try_impact(
    file: &Path, format: &str, json: bool,
    change_impact: &Option<String>, blast_radius: &Option<String>,
    what_if: &Option<String>, timeline_graph: bool, hotspots: bool,
    dependency_matrix: bool, impact_radius: &Option<String>,
    subgraph: &Option<String>, weight: bool,
) -> Option<Result<(), String>> {
    if let Some(ref r) = change_impact { return Some(cmd_graph_change_impact(file, r, json)); }
    if let Some(ref r) = blast_radius { return Some(cmd_graph_blast_radius(file, r, json)); }
    if let Some(ref r) = what_if { return Some(cmd_graph_what_if(file, r)); }
    if timeline_graph { return Some(cmd_graph_timeline(file)); }
    if hotspots { return Some(cmd_graph_hotspots(file)); }
    if dependency_matrix { return Some(cmd_graph_dependency_matrix(file, json)); }
    if let Some(ref r) = impact_radius { return Some(cmd_graph_impact_radius(file, r)); }
    if let Some(ref r) = subgraph { return Some(cmd_graph_subgraph(file, format, r)); }
    if weight { return Some(cmd_graph_weight(file, format)); }
    None
}

/// Dispatch visualization/filter flags.
fn try_visualization(
    file: &Path, format: &str, json: bool,
    critical_resources: bool, layers: bool,
    prune: &Option<String>, highlight: &Option<String>,
    stats: bool, orphans: bool, cluster: bool, reverse: bool,
    critical_path: bool, affected: &Option<String>, depth: Option<usize>,
) -> Option<Result<(), String>> {
    if critical_resources { return Some(cmd_graph_critical_resources(file)); }
    if layers { return Some(cmd_graph_layers(file)); }
    if let Some(ref r) = prune { return Some(cmd_graph_prune(file, format, r)); }
    if let Some(ref r) = highlight { return Some(cmd_graph_highlight(file, format, r)); }
    if json { return Some(cmd_graph_json(file)); }
    if stats { return Some(cmd_graph_stats(file)); }
    if orphans { return Some(cmd_graph_orphans(file)); }
    if cluster { return Some(cmd_graph_cluster(file, format)); }
    if reverse { return Some(cmd_graph_reverse(file)); }
    if critical_path { return Some(cmd_graph_critical_path(file)); }
    if let Some(ref r) = affected { return Some(cmd_graph_affected(file, r)); }
    if let Some(d) = depth { return Some(cmd_graph_depth(file, format, d)); }
    None
}

/// Dispatch the Graph command variant.
pub(crate) fn dispatch_graph_cmd(cmd: Commands) -> Result<(), String> {
    let Commands::Graph(GraphArgs {
        file, format, machine, group,
        affected, critical_path, reverse, depth, cluster,
        orphans, stats, json_output, highlight, prune, layers,
        critical_resources, weight, subgraph, impact_radius,
        dependency_matrix, hotspots, timeline_graph,
        what_if, blast_radius, change_impact, resource_types,
        topological_levels, execution_order, security_boundaries,
        resource_age, parallel_groups, critical_chain,
        dependency_depth, orphan_detection, cross_machine_deps,
        machine_groups, resource_clusters, fan_out, leaf_resources,
        reverse_deps, depth_first, breadth_first,
        subgraph_stats, dependency_count: graph_dependency_count,
        root_resources, edge_list,
        connected_components, adjacency_matrix,
        longest_path, in_degree,
        out_degree, density,
        topological_sort, critical_path_resources,
        sink_resources, bipartite_check,
        strongly_connected, dependency_matrix_csv,
        resource_weight, dependency_depth_per_resource,
    }) = cmd
    else {
        unreachable!()
    };

    if breadth_first { return cmd_graph_breadth_first(&file, json_output); }
    if subgraph_stats { return cmd_graph_subgraph_stats(&file, json_output); }
    if graph_dependency_count { return cmd_graph_dependency_count(&file, json_output); }
    if root_resources { return cmd_graph_root_resources(&file, json_output); }
    if edge_list { return cmd_graph_edge_list(&file, json_output); }
    if connected_components { return cmd_graph_connected_components(&file, json_output); }
    if adjacency_matrix { return cmd_graph_adjacency_matrix(&file, json_output); }
    if longest_path { return cmd_graph_longest_path(&file, json_output); }
    if in_degree { return cmd_graph_in_degree(&file, json_output); }
    if out_degree { return cmd_graph_out_degree(&file, json_output); }
    if density { return cmd_graph_density(&file, json_output); }
    if topological_sort { return cmd_graph_topological_sort(&file, json_output); }
    if critical_path_resources { return cmd_graph_critical_path_resources(&file, json_output); }
    if sink_resources { return cmd_graph_sink_resources(&file, json_output); }
    if bipartite_check { return cmd_graph_bipartite_check(&file, json_output); }
    if strongly_connected { return cmd_graph_strongly_connected(&file, json_output); }
    if dependency_matrix_csv { return cmd_graph_dependency_matrix_csv(&file, json_output); }
    if resource_weight { return cmd_graph_resource_weight(&file, json_output); }
    if dependency_depth_per_resource { return cmd_graph_dependency_depth_per_resource(&file, json_output); }
    if let Some(r) = try_traversal(&file, json_output, depth_first, reverse_deps, leaf_resources, fan_out, resource_clusters, machine_groups, cross_machine_deps, orphan_detection) {
        return r;
    }
    if let Some(r) = try_topology(&file, json_output, dependency_depth, critical_chain, parallel_groups, resource_age, security_boundaries, execution_order, topological_levels, resource_types) {
        return r;
    }
    if let Some(r) = try_impact(&file, &format, json_output, &change_impact, &blast_radius, &what_if, timeline_graph, hotspots, dependency_matrix, &impact_radius, &subgraph, weight) {
        return r;
    }
    if let Some(r) = try_visualization(&file, &format, json_output, critical_resources, layers, &prune, &highlight, stats, orphans, cluster, reverse, critical_path, &affected, depth) {
        return r;
    }
    cmd_graph(&file, &format, machine.as_deref(), group.as_deref())
}
