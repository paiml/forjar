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
use super::graph_paths::*;
use super::graph_scoring::*;
use super::graph_intelligence::*;
use super::graph_intelligence_ext::*;
use super::graph_intelligence_ext2::*;
use super::graph_resilience::*;
use super::graph_transport::*;


/// Dispatch traversal flags (depth_first through critical_chain).
#[allow(clippy::too_many_arguments)]
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
#[allow(clippy::too_many_arguments)]
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
#[allow(clippy::too_many_arguments)]
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
#[allow(clippy::too_many_arguments)]
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

/// Phase 70-73 graph path/scoring flags.
#[allow(clippy::too_many_arguments)]
fn try_graph_paths(
    file: &Path, json: bool,
    resource_dependency_chain: &Option<String>, bottleneck_resources: bool,
    critical_dependency_path: bool, resource_depth_histogram: bool,
    resource_coupling_score: bool, resource_change_frequency: bool,
    resource_impact_score: bool, resource_stability_score: bool,
    resource_dependency_fanout: bool, resource_dependency_weight: bool,
) -> Option<Result<(), String>> {
    if let Some(ref target) = resource_dependency_chain { return Some(cmd_graph_resource_dependency_chain(file, target, json)); }
    if bottleneck_resources { return Some(cmd_graph_bottleneck_resources(file, json)); }
    if critical_dependency_path { return Some(cmd_graph_critical_dependency_path(file, json)); }
    if resource_depth_histogram { return Some(cmd_graph_resource_depth_histogram(file, json)); }
    if resource_coupling_score { return Some(cmd_graph_resource_coupling_score(file, json)); }
    if resource_change_frequency { return Some(cmd_graph_resource_change_frequency(file, json)); }
    if resource_impact_score { return Some(cmd_graph_resource_impact_score(file, json)); }
    if resource_stability_score { return Some(cmd_graph_resource_stability_score(file, json)); }
    if resource_dependency_fanout { return Some(cmd_graph_resource_dependency_fanout(file, json)); }
    if resource_dependency_weight { return Some(cmd_graph_resource_dependency_weight(file, json)); }
    None
}

/// Phase 66-69 graph analysis flags.
#[allow(clippy::too_many_arguments)]
fn try_graph_analysis(
    file: &Path, json: bool,
    resource_weight: bool, dependency_depth_per_resource: bool,
    resource_fanin: bool, isolated_subgraphs: bool,
    dependency_matrix_csv: bool, strongly_connected: bool,
    bipartite_check: bool, sink_resources: bool,
) -> Option<Result<(), String>> {
    if resource_weight { return Some(cmd_graph_resource_weight(file, json)); }
    if dependency_depth_per_resource { return Some(cmd_graph_dependency_depth_per_resource(file, json)); }
    if resource_fanin { return Some(cmd_graph_resource_fanin(file, json)); }
    if isolated_subgraphs { return Some(cmd_graph_isolated_subgraphs(file, json)); }
    if dependency_matrix_csv { return Some(cmd_graph_dependency_matrix_csv(file, json)); }
    if strongly_connected { return Some(cmd_graph_strongly_connected(file, json)); }
    if bipartite_check { return Some(cmd_graph_bipartite_check(file, json)); }
    if sink_resources { return Some(cmd_graph_sink_resources(file, json)); }
    None
}

/// Phase 63-64 graph export flags (part A).
#[allow(clippy::too_many_arguments)]
fn try_graph_export_a(
    file: &Path, json: bool,
    breadth_first: bool, subgraph_stats: bool, graph_dependency_count: bool,
    root_resources: bool, edge_list: bool, connected_components: bool,
    adjacency_matrix: bool,
) -> Option<Result<(), String>> {
    if breadth_first { return Some(cmd_graph_breadth_first(file, json)); }
    if subgraph_stats { return Some(cmd_graph_subgraph_stats(file, json)); }
    if graph_dependency_count { return Some(cmd_graph_dependency_count(file, json)); }
    if root_resources { return Some(cmd_graph_root_resources(file, json)); }
    if edge_list { return Some(cmd_graph_edge_list(file, json)); }
    if connected_components { return Some(cmd_graph_connected_components(file, json)); }
    if adjacency_matrix { return Some(cmd_graph_adjacency_matrix(file, json)); }
    None
}

/// Phase 64-65 graph export flags (part B).
#[allow(clippy::too_many_arguments)]
fn try_graph_export_b(
    file: &Path, json: bool,
    longest_path: bool, in_degree: bool, out_degree: bool,
    density: bool, topological_sort: bool, critical_path_resources: bool,
) -> Option<Result<(), String>> {
    if longest_path { return Some(cmd_graph_longest_path(file, json)); }
    if in_degree { return Some(cmd_graph_in_degree(file, json)); }
    if out_degree { return Some(cmd_graph_out_degree(file, json)); }
    if density { return Some(cmd_graph_density(file, json)); }
    if topological_sort { return Some(cmd_graph_topological_sort(file, json)); }
    if critical_path_resources { return Some(cmd_graph_critical_path_resources(file, json)); }
    None
}

/// Phase 75-77 scoring graph flags.
#[allow(clippy::too_many_arguments)]
fn try_graph_scoring_inline(
    file: &Path, json: bool,
    resource_dependency_bottleneck: bool, resource_type_clustering: bool,
    resource_dependency_cycle_risk: bool, resource_impact_radius: bool,
    resource_dependency_health_map: bool, resource_change_propagation: bool,
    resource_dependency_depth_analysis: bool, resource_dependency_fan_analysis: bool,
    resource_dependency_isolation_score: bool, resource_dependency_stability_score: bool,
    resource_dependency_critical_path_length: bool, resource_dependency_redundancy_score: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_bottleneck { return Some(cmd_graph_resource_dependency_bottleneck(file, json)); }
    if resource_type_clustering { return Some(cmd_graph_resource_type_clustering(file, json)); }
    if resource_dependency_cycle_risk { return Some(cmd_graph_resource_dependency_cycle_risk(file, json)); }
    if resource_impact_radius { return Some(cmd_graph_resource_impact_radius_analysis(file, json)); }
    if resource_dependency_health_map { return Some(cmd_graph_resource_dependency_health_map(file, json)); }
    if resource_change_propagation { return Some(cmd_graph_resource_change_propagation(file, json)); }
    if resource_dependency_depth_analysis { return Some(cmd_graph_resource_dependency_depth_analysis(file, json)); }
    if resource_dependency_fan_analysis { return Some(cmd_graph_resource_dependency_fan_analysis(file, json)); }
    if resource_dependency_isolation_score { return Some(cmd_graph_resource_dependency_isolation_score(file, json)); }
    if resource_dependency_stability_score { return Some(cmd_graph_resource_dependency_stability_score(file, json)); }
    if resource_dependency_critical_path_length { return Some(cmd_graph_resource_dependency_critical_path_length(file, json)); }
    if resource_dependency_redundancy_score { return Some(cmd_graph_resource_dependency_redundancy_score(file, json)); }
    None
}

/// Phase 81-84 scoring graph flags.
#[allow(clippy::too_many_arguments)]
fn try_graph_scoring_phase81(
    file: &Path, json: bool,
    resource_dependency_centrality_score: bool, resource_dependency_bridge_detection: bool,
    resource_dependency_cluster_coefficient: bool, resource_dependency_modularity_score: bool,
    resource_dependency_diameter: bool, resource_dependency_eccentricity: bool,
    resource_dependency_density: bool, resource_dependency_transitivity: bool,
    resource_dependency_fan_out: bool, resource_dependency_fan_in: bool,
    resource_dependency_path_count: bool, resource_dependency_articulation_points: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_centrality_score { return Some(cmd_graph_resource_dependency_centrality_score(file, json)); }
    if resource_dependency_bridge_detection { return Some(cmd_graph_resource_dependency_bridge_detection(file, json)); }
    if resource_dependency_cluster_coefficient { return Some(cmd_graph_resource_dependency_cluster_coefficient(file, json)); }
    if resource_dependency_modularity_score { return Some(cmd_graph_resource_dependency_modularity_score(file, json)); }
    if resource_dependency_diameter { return Some(cmd_graph_resource_dependency_diameter(file, json)); }
    if resource_dependency_eccentricity { return Some(cmd_graph_resource_dependency_eccentricity(file, json)); }
    if resource_dependency_density { return Some(cmd_graph_resource_dependency_density(file, json)); }
    if resource_dependency_transitivity { return Some(cmd_graph_resource_dependency_transitivity(file, json)); }
    if resource_dependency_fan_out { return Some(cmd_graph_resource_dependency_fan_out(file, json)); }
    if resource_dependency_fan_in { return Some(cmd_graph_resource_dependency_fan_in(file, json)); }
    if resource_dependency_path_count { return Some(cmd_graph_resource_dependency_path_count(file, json)); }
    if resource_dependency_articulation_points { return Some(cmd_graph_resource_dependency_articulation_points(file, json)); }
    None
}

/// Phase 87–90 graph analysis flags.
#[allow(clippy::too_many_arguments)]
fn try_graph_phase87(
    file: &Path, json: bool,
    resource_dependency_longest_path: bool, resource_dependency_strongly_connected: bool,
    resource_dependency_topological_depth: bool, resource_dependency_weak_links: bool,
    resource_dependency_minimum_cut: bool, resource_dependency_dominator_tree: bool,
    resource_dependency_resilience_score: bool, resource_dependency_pagerank: bool,
    resource_dependency_betweenness_centrality: bool, resource_dependency_closure_size: bool,
    resource_dependency_eccentricity_map: bool, resource_dependency_diameter_path: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_longest_path { return Some(cmd_graph_resource_dependency_longest_path(file, json)); }
    if resource_dependency_strongly_connected { return Some(cmd_graph_resource_dependency_strongly_connected(file, json)); }
    if resource_dependency_topological_depth { return Some(cmd_graph_resource_dependency_topological_depth(file, json)); }
    if resource_dependency_weak_links { return Some(cmd_graph_resource_dependency_weak_links(file, json)); }
    if resource_dependency_minimum_cut { return Some(cmd_graph_resource_dependency_minimum_cut(file, json)); }
    if resource_dependency_dominator_tree { return Some(cmd_graph_resource_dependency_dominator_tree(file, json)); }
    if resource_dependency_resilience_score { return Some(cmd_graph_resource_dependency_resilience_score(file, json)); }
    if resource_dependency_pagerank { return Some(cmd_graph_resource_dependency_pagerank(file, json)); }
    if resource_dependency_betweenness_centrality { return Some(cmd_graph_resource_dependency_betweenness_centrality(file, json)); }
    if resource_dependency_closure_size { return Some(cmd_graph_resource_dependency_closure_size(file, json)); }
    if resource_dependency_eccentricity_map { return Some(cmd_graph_resource_dependency_eccentricity_map(file, json)); }
    if resource_dependency_diameter_path { return Some(cmd_graph_resource_dependency_diameter_path(file, json)); }
    None
}

/// Phase 94 graph analysis flags.
fn try_graph_phase94(
    file: &Path, json: bool,
    resource_dependency_bridge_criticality: bool, resource_dependency_conditional_subgraph: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_bridge_criticality { return Some(cmd_graph_resource_dependency_bridge_criticality(file, json)); }
    if resource_dependency_conditional_subgraph { return Some(cmd_graph_resource_dependency_conditional_subgraph(file, json)); }
    None
}
fn try_graph_phase95(
    file: &Path, json: bool,
    resource_dependency_parallel_groups: bool, resource_dependency_execution_cost: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_parallel_groups { return Some(cmd_graph_resource_dependency_parallel_groups(file, json)); }
    if resource_dependency_execution_cost { return Some(cmd_graph_resource_dependency_execution_cost(file, json)); }
    None
}
fn try_graph_phase96(
    file: &Path, json: bool,
    resource_recipe_expansion_map: bool, resource_dependency_critical_chain_path: bool,
) -> Option<Result<(), String>> {
    if resource_recipe_expansion_map { return Some(cmd_graph_resource_recipe_expansion_map(file, json)); }
    if resource_dependency_critical_chain_path { return Some(cmd_graph_resource_dependency_critical_chain_path(file, json)); }
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
        resource_fanin, isolated_subgraphs,
        resource_dependency_chain, bottleneck_resources,
        critical_dependency_path, resource_depth_histogram,
        resource_coupling_score, resource_change_frequency,
        resource_impact_score, resource_stability_score,
        resource_dependency_fanout, resource_dependency_weight,
        resource_dependency_bottleneck, resource_type_clustering,
        resource_dependency_cycle_risk, resource_impact_radius,
        resource_dependency_health_map, resource_change_propagation,
        resource_dependency_depth_analysis, resource_dependency_fan_analysis,
        resource_dependency_isolation_score, resource_dependency_stability_score,
        resource_dependency_critical_path_length, resource_dependency_redundancy_score,
        resource_dependency_centrality_score, resource_dependency_bridge_detection,
        resource_dependency_cluster_coefficient, resource_dependency_modularity_score,
        resource_dependency_diameter, resource_dependency_eccentricity,
        resource_dependency_density, resource_dependency_transitivity,
        resource_dependency_fan_out, resource_dependency_fan_in,
        resource_dependency_path_count, resource_dependency_articulation_points,
        resource_dependency_longest_path, resource_dependency_strongly_connected,
        resource_dependency_topological_depth, resource_dependency_weak_links,
        resource_dependency_minimum_cut, resource_dependency_dominator_tree,
        resource_dependency_resilience_score, resource_dependency_pagerank,
        resource_dependency_betweenness_centrality, resource_dependency_closure_size,
        resource_dependency_eccentricity_map, resource_dependency_diameter_path,
        resource_dependency_bridge_criticality, resource_dependency_conditional_subgraph,
        resource_dependency_parallel_groups: resource_dependency_parallel_groups_p95,
        resource_dependency_execution_cost,
        resource_recipe_expansion_map,
        resource_dependency_critical_chain_path,
    }) = cmd
    else {
        unreachable!()
    };

    if let Some(r) = try_graph_export_a(&file, json_output, breadth_first, subgraph_stats, graph_dependency_count, root_resources, edge_list, connected_components, adjacency_matrix) {
        return r;
    }
    if let Some(r) = try_graph_export_b(&file, json_output, longest_path, in_degree, out_degree, density, topological_sort, critical_path_resources) {
        return r;
    }
    if let Some(r) = try_graph_scoring_phase81(&file, json_output, resource_dependency_centrality_score, resource_dependency_bridge_detection, resource_dependency_cluster_coefficient, resource_dependency_modularity_score, resource_dependency_diameter, resource_dependency_eccentricity, resource_dependency_density, resource_dependency_transitivity, resource_dependency_fan_out, resource_dependency_fan_in, resource_dependency_path_count, resource_dependency_articulation_points) {
        return r;
    }
    if let Some(r) = try_graph_phase87(&file, json_output, resource_dependency_longest_path, resource_dependency_strongly_connected, resource_dependency_topological_depth, resource_dependency_weak_links, resource_dependency_minimum_cut, resource_dependency_dominator_tree, resource_dependency_resilience_score, resource_dependency_pagerank, resource_dependency_betweenness_centrality, resource_dependency_closure_size, resource_dependency_eccentricity_map, resource_dependency_diameter_path) {
        return r;
    }
    if let Some(r) = try_graph_phase94(&file, json_output, resource_dependency_bridge_criticality, resource_dependency_conditional_subgraph) {
        return r;
    }
    if let Some(r) = try_graph_phase95(&file, json_output, resource_dependency_parallel_groups_p95, resource_dependency_execution_cost) {
        return r;
    }
    if let Some(r) = try_graph_phase96(&file, json_output, resource_recipe_expansion_map, resource_dependency_critical_chain_path) {
        return r;
    }
    if let Some(r) = try_graph_scoring_inline(&file, json_output, resource_dependency_bottleneck, resource_type_clustering, resource_dependency_cycle_risk, resource_impact_radius, resource_dependency_health_map, resource_change_propagation, resource_dependency_depth_analysis, resource_dependency_fan_analysis, resource_dependency_isolation_score, resource_dependency_stability_score, resource_dependency_critical_path_length, resource_dependency_redundancy_score) {
        return r;
    }
    if let Some(r) = try_graph_paths(&file, json_output, &resource_dependency_chain, bottleneck_resources, critical_dependency_path, resource_depth_histogram, resource_coupling_score, resource_change_frequency, resource_impact_score, resource_stability_score, resource_dependency_fanout, resource_dependency_weight) {
        return r;
    }
    if let Some(r) = try_graph_analysis(&file, json_output, resource_weight, dependency_depth_per_resource, resource_fanin, isolated_subgraphs, dependency_matrix_csv, strongly_connected, bipartite_check, sink_resources) {
        return r;
    }
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
