use super::commands::*;
use super::dispatch_graph::*;
use super::graph_core::*;
use super::graph_quality::*;

pub(crate) fn dispatch_graph_cmd(cmd: Commands) -> Result<(), String> {
    let Commands::Graph(args) = cmd else {
        unreachable!()
    };
    if let Some(r) = try_graph_group_early(&args) {
        return r;
    }
    if let Some(r) = try_graph_group_middle(&args) {
        return r;
    }
    if let Some(r) = try_graph_group_late(&args) {
        return r;
    }
    cmd_graph(
        &args.file,
        &args.format,
        args.machine.as_deref(),
        args.group.as_deref(),
    )
}

#[allow(clippy::too_many_lines)]
fn try_graph_group_early(a: &GraphArgs) -> Option<Result<(), String>> {
    try_graph_export_a(
        &a.file,
        a.json_output,
        a.breadth_first,
        a.subgraph_stats,
        a.dependency_count,
        a.root_resources,
        a.edge_list,
        a.connected_components,
        a.adjacency_matrix,
    )
    .or_else(|| {
        try_graph_export_b(
            &a.file,
            a.json_output,
            a.longest_path,
            a.in_degree,
            a.out_degree,
            a.density,
            a.topological_sort,
            a.critical_path_resources,
        )
    })
    .or_else(|| {
        try_graph_scoring_phase81(
            &a.file,
            a.json_output,
            a.resource_dependency_centrality_score,
            a.resource_dependency_bridge_detection,
            a.resource_dependency_cluster_coefficient,
            a.resource_dependency_modularity_score,
            a.resource_dependency_diameter,
            a.resource_dependency_eccentricity,
            a.resource_dependency_density,
            a.resource_dependency_transitivity,
            a.resource_dependency_fan_out,
            a.resource_dependency_fan_in,
            a.resource_dependency_path_count,
            a.resource_dependency_articulation_points,
        )
    })
    .or_else(|| {
        try_graph_phase87(
            &a.file,
            a.json_output,
            a.resource_dependency_longest_path,
            a.resource_dependency_strongly_connected,
            a.resource_dependency_topological_depth,
            a.resource_dependency_weak_links,
            a.resource_dependency_minimum_cut,
            a.resource_dependency_dominator_tree,
            a.resource_dependency_resilience_score,
            a.resource_dependency_pagerank,
            a.resource_dependency_betweenness_centrality,
            a.resource_dependency_closure_size,
            a.resource_dependency_eccentricity_map,
            a.resource_dependency_diameter_path,
        )
    })
    .or_else(|| {
        try_graph_phase94(
            &a.file,
            a.json_output,
            a.resource_dependency_bridge_criticality,
            a.resource_dependency_conditional_subgraph,
        )
    })
    .or_else(|| {
        try_graph_phase95(
            &a.file,
            a.json_output,
            a.resource_dependency_parallel_groups,
            a.resource_dependency_execution_cost,
        )
    })
    .or_else(|| {
        try_graph_phase96(
            &a.file,
            a.json_output,
            a.resource_recipe_expansion_map,
            a.resource_dependency_critical_chain_path,
        )
    })
    .or_else(|| {
        try_graph_phase97(
            &a.file,
            a.json_output,
            a.resource_apply_order_simulation,
            a.resource_provenance_summary,
        )
    })
}

fn try_graph_group_middle(a: &GraphArgs) -> Option<Result<(), String>> {
    try_graph_phase98(
        &a.file,
        a.json_output,
        a.resource_dependency_risk_score,
        a.resource_dependency_layering,
    )
    .or_else(|| {
        try_graph_phase99(
            &a.file,
            a.json_output,
            a.resource_lifecycle_stage_map,
            a.resource_dependency_age_overlay,
        )
    })
    .or_else(|| {
        try_graph_phase100(
            &a.file,
            a.json_output,
            a.resource_dependency_health_overlay,
            a.resource_dependency_width_analysis,
        )
    })
    .or_else(|| {
        try_graph_phase101(
            &a.file,
            a.json_output,
            a.resource_dependency_critical_path_highlight,
            a.resource_dependency_bottleneck_detection,
        )
    })
    .or_else(|| {
        try_graph_phase102(
            &a.file,
            a.json_output,
            a.resource_topology_cluster_analysis,
            a.resource_dependency_island_detection,
        )
    })
    .or_else(|| {
        try_graph_phase103(
            &a.file,
            a.json_output,
            a.resource_dependency_depth_histogram_analysis,
            a.resource_dependency_redundancy_analysis,
        )
    })
    .or_else(|| {
        try_graph_phases_104_106(
            &a.file,
            a.json_output,
            a.resource_dependency_change_impact_radius,
            a.resource_dependency_sibling_analysis,
            a.resource_dependency_fan_in_hotspot,
            a.resource_dependency_cross_machine_bridge,
            a.resource_dependency_weight_analysis,
            a.resource_dependency_topological_summary,
        )
    })
    .or_else(|| {
        if a.resource_dependency_critical_path {
            Some(cmd_graph_resource_dependency_critical_path(
                &a.file,
                a.json_output,
            ))
        } else {
            None
        }
    })
    .or_else(|| {
        if a.resource_dependency_cluster_analysis {
            Some(cmd_graph_resource_dependency_cluster_analysis(
                &a.file,
                a.json_output,
            ))
        } else {
            None
        }
    })
}

#[allow(clippy::too_many_lines)]
fn try_graph_group_late(a: &GraphArgs) -> Option<Result<(), String>> {
    try_graph_scoring_inline(
        &a.file,
        a.json_output,
        a.resource_dependency_bottleneck,
        a.resource_type_clustering,
        a.resource_dependency_cycle_risk,
        a.resource_impact_radius,
        a.resource_dependency_health_map,
        a.resource_change_propagation,
        a.resource_dependency_depth_analysis,
        a.resource_dependency_fan_analysis,
        a.resource_dependency_isolation_score,
        a.resource_dependency_stability_score,
        a.resource_dependency_critical_path_length,
        a.resource_dependency_redundancy_score,
    )
    .or_else(|| {
        try_graph_paths(
            &a.file,
            a.json_output,
            &a.resource_dependency_chain,
            a.bottleneck_resources,
            a.critical_dependency_path,
            a.resource_depth_histogram,
            a.resource_coupling_score,
            a.resource_change_frequency,
            a.resource_impact_score,
            a.resource_stability_score,
            a.resource_dependency_fanout,
            a.resource_dependency_weight,
        )
    })
    .or_else(|| {
        try_graph_analysis(
            &a.file,
            a.json_output,
            a.resource_weight,
            a.dependency_depth_per_resource,
            a.resource_fanin,
            a.isolated_subgraphs,
            a.dependency_matrix_csv,
            a.strongly_connected,
            a.bipartite_check,
            a.sink_resources,
        )
    })
    .or_else(|| {
        try_traversal(
            &a.file,
            a.json_output,
            a.depth_first,
            a.reverse_deps,
            a.leaf_resources,
            a.fan_out,
            a.resource_clusters,
            a.machine_groups,
            a.cross_machine_deps,
            a.orphan_detection,
        )
    })
    .or_else(|| {
        try_topology(
            &a.file,
            a.json_output,
            a.dependency_depth,
            a.critical_chain,
            a.parallel_groups,
            a.resource_age,
            a.security_boundaries,
            a.execution_order,
            a.topological_levels,
            a.resource_types,
        )
    })
    .or_else(|| {
        try_impact(
            &a.file,
            &a.format,
            a.json_output,
            &a.change_impact,
            &a.blast_radius,
            &a.what_if,
            a.timeline_graph,
            a.hotspots,
            a.dependency_matrix,
            &a.impact_radius,
            &a.subgraph,
            a.weight,
        )
    })
    .or_else(|| {
        try_visualization(
            &a.file,
            &a.format,
            a.json_output,
            a.critical_resources,
            a.layers,
            &a.prune,
            &a.highlight,
            a.stats,
            a.orphans,
            a.cluster,
            a.reverse,
            a.critical_path,
            &a.affected,
            a.depth,
        )
    })
}
