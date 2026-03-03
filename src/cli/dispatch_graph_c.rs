use super::commands::*;
use super::dispatch_graph::*;
use super::graph_core::*;
use super::graph_quality::*;

pub(crate) fn dispatch_graph_cmd(cmd: Commands) -> Result<(), String> {
    let Commands::Graph(GraphArgs {
        file,
        format,
        machine,
        group,
        affected,
        critical_path,
        reverse,
        depth,
        cluster,
        orphans,
        stats,
        json_output,
        highlight,
        prune,
        layers,
        critical_resources,
        weight,
        subgraph,
        impact_radius,
        dependency_matrix,
        hotspots,
        timeline_graph,
        what_if,
        blast_radius,
        change_impact,
        resource_types,
        topological_levels,
        execution_order,
        security_boundaries,
        resource_age,
        parallel_groups,
        critical_chain,
        dependency_depth,
        orphan_detection,
        cross_machine_deps,
        machine_groups,
        resource_clusters,
        fan_out,
        leaf_resources,
        reverse_deps,
        depth_first,
        breadth_first,
        subgraph_stats,
        dependency_count: graph_dependency_count,
        root_resources,
        edge_list,
        connected_components,
        adjacency_matrix,
        longest_path,
        in_degree,
        out_degree,
        density,
        topological_sort,
        critical_path_resources,
        sink_resources,
        bipartite_check,
        strongly_connected,
        dependency_matrix_csv,
        resource_weight,
        dependency_depth_per_resource,
        resource_fanin,
        isolated_subgraphs,
        resource_dependency_chain,
        bottleneck_resources,
        critical_dependency_path,
        resource_depth_histogram,
        resource_coupling_score,
        resource_change_frequency,
        resource_impact_score,
        resource_stability_score,
        resource_dependency_fanout,
        resource_dependency_weight,
        resource_dependency_bottleneck,
        resource_type_clustering,
        resource_dependency_cycle_risk,
        resource_impact_radius,
        resource_dependency_health_map,
        resource_change_propagation,
        resource_dependency_depth_analysis,
        resource_dependency_fan_analysis,
        resource_dependency_isolation_score,
        resource_dependency_stability_score,
        resource_dependency_critical_path_length,
        resource_dependency_redundancy_score,
        resource_dependency_centrality_score,
        resource_dependency_bridge_detection,
        resource_dependency_cluster_coefficient,
        resource_dependency_modularity_score,
        resource_dependency_diameter,
        resource_dependency_eccentricity,
        resource_dependency_density,
        resource_dependency_transitivity,
        resource_dependency_fan_out,
        resource_dependency_fan_in,
        resource_dependency_path_count,
        resource_dependency_articulation_points,
        resource_dependency_longest_path,
        resource_dependency_strongly_connected,
        resource_dependency_topological_depth,
        resource_dependency_weak_links,
        resource_dependency_minimum_cut,
        resource_dependency_dominator_tree,
        resource_dependency_resilience_score,
        resource_dependency_pagerank,
        resource_dependency_betweenness_centrality,
        resource_dependency_closure_size,
        resource_dependency_eccentricity_map,
        resource_dependency_diameter_path,
        resource_dependency_bridge_criticality,
        resource_dependency_conditional_subgraph,
        resource_dependency_parallel_groups: resource_dependency_parallel_groups_p95,
        resource_dependency_execution_cost,
        resource_recipe_expansion_map,
        resource_dependency_critical_chain_path,
        resource_apply_order_simulation,
        resource_provenance_summary,
        resource_dependency_risk_score,
        resource_dependency_layering,
        resource_lifecycle_stage_map,
        resource_dependency_age_overlay,
        resource_dependency_health_overlay,
        resource_dependency_width_analysis,
        resource_dependency_critical_path_highlight,
        resource_dependency_bottleneck_detection,
        resource_topology_cluster_analysis,
        resource_dependency_island_detection,
        resource_dependency_depth_histogram_analysis,
        resource_dependency_redundancy_analysis,
        resource_dependency_change_impact_radius,
        resource_dependency_sibling_analysis,
        resource_dependency_fan_in_hotspot,
        resource_dependency_cross_machine_bridge,
        resource_dependency_weight_analysis,
        resource_dependency_topological_summary,
        resource_dependency_critical_path: rdcp107,
        resource_dependency_cluster_analysis: rdca107,
    }) = cmd
    else {
        unreachable!()
    };

    if let Some(r) = try_graph_export_a(
        &file,
        json_output,
        breadth_first,
        subgraph_stats,
        graph_dependency_count,
        root_resources,
        edge_list,
        connected_components,
        adjacency_matrix,
    ) {
        return r;
    }
    if let Some(r) = try_graph_export_b(
        &file,
        json_output,
        longest_path,
        in_degree,
        out_degree,
        density,
        topological_sort,
        critical_path_resources,
    ) {
        return r;
    }
    if let Some(r) = try_graph_scoring_phase81(
        &file,
        json_output,
        resource_dependency_centrality_score,
        resource_dependency_bridge_detection,
        resource_dependency_cluster_coefficient,
        resource_dependency_modularity_score,
        resource_dependency_diameter,
        resource_dependency_eccentricity,
        resource_dependency_density,
        resource_dependency_transitivity,
        resource_dependency_fan_out,
        resource_dependency_fan_in,
        resource_dependency_path_count,
        resource_dependency_articulation_points,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase87(
        &file,
        json_output,
        resource_dependency_longest_path,
        resource_dependency_strongly_connected,
        resource_dependency_topological_depth,
        resource_dependency_weak_links,
        resource_dependency_minimum_cut,
        resource_dependency_dominator_tree,
        resource_dependency_resilience_score,
        resource_dependency_pagerank,
        resource_dependency_betweenness_centrality,
        resource_dependency_closure_size,
        resource_dependency_eccentricity_map,
        resource_dependency_diameter_path,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase94(
        &file,
        json_output,
        resource_dependency_bridge_criticality,
        resource_dependency_conditional_subgraph,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase95(
        &file,
        json_output,
        resource_dependency_parallel_groups_p95,
        resource_dependency_execution_cost,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase96(
        &file,
        json_output,
        resource_recipe_expansion_map,
        resource_dependency_critical_chain_path,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase97(
        &file,
        json_output,
        resource_apply_order_simulation,
        resource_provenance_summary,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase98(
        &file,
        json_output,
        resource_dependency_risk_score,
        resource_dependency_layering,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase99(
        &file,
        json_output,
        resource_lifecycle_stage_map,
        resource_dependency_age_overlay,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase100(
        &file,
        json_output,
        resource_dependency_health_overlay,
        resource_dependency_width_analysis,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase101(
        &file,
        json_output,
        resource_dependency_critical_path_highlight,
        resource_dependency_bottleneck_detection,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase102(
        &file,
        json_output,
        resource_topology_cluster_analysis,
        resource_dependency_island_detection,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phase103(
        &file,
        json_output,
        resource_dependency_depth_histogram_analysis,
        resource_dependency_redundancy_analysis,
    ) {
        return r;
    }
    if let Some(r) = try_graph_phases_104_106(
        &file,
        json_output,
        resource_dependency_change_impact_radius,
        resource_dependency_sibling_analysis,
        resource_dependency_fan_in_hotspot,
        resource_dependency_cross_machine_bridge,
        resource_dependency_weight_analysis,
        resource_dependency_topological_summary,
    ) {
        return r;
    }
    if rdcp107 {
        return cmd_graph_resource_dependency_critical_path(&file, json_output);
    }
    if rdca107 {
        return cmd_graph_resource_dependency_cluster_analysis(&file, json_output);
    }
    if let Some(r) = try_graph_scoring_inline(
        &file,
        json_output,
        resource_dependency_bottleneck,
        resource_type_clustering,
        resource_dependency_cycle_risk,
        resource_impact_radius,
        resource_dependency_health_map,
        resource_change_propagation,
        resource_dependency_depth_analysis,
        resource_dependency_fan_analysis,
        resource_dependency_isolation_score,
        resource_dependency_stability_score,
        resource_dependency_critical_path_length,
        resource_dependency_redundancy_score,
    ) {
        return r;
    }
    if let Some(r) = try_graph_paths(
        &file,
        json_output,
        &resource_dependency_chain,
        bottleneck_resources,
        critical_dependency_path,
        resource_depth_histogram,
        resource_coupling_score,
        resource_change_frequency,
        resource_impact_score,
        resource_stability_score,
        resource_dependency_fanout,
        resource_dependency_weight,
    ) {
        return r;
    }
    if let Some(r) = try_graph_analysis(
        &file,
        json_output,
        resource_weight,
        dependency_depth_per_resource,
        resource_fanin,
        isolated_subgraphs,
        dependency_matrix_csv,
        strongly_connected,
        bipartite_check,
        sink_resources,
    ) {
        return r;
    }
    if let Some(r) = try_traversal(
        &file,
        json_output,
        depth_first,
        reverse_deps,
        leaf_resources,
        fan_out,
        resource_clusters,
        machine_groups,
        cross_machine_deps,
        orphan_detection,
    ) {
        return r;
    }
    if let Some(r) = try_topology(
        &file,
        json_output,
        dependency_depth,
        critical_chain,
        parallel_groups,
        resource_age,
        security_boundaries,
        execution_order,
        topological_levels,
        resource_types,
    ) {
        return r;
    }
    if let Some(r) = try_impact(
        &file,
        &format,
        json_output,
        &change_impact,
        &blast_radius,
        &what_if,
        timeline_graph,
        hotspots,
        dependency_matrix,
        &impact_radius,
        &subgraph,
        weight,
    ) {
        return r;
    }
    if let Some(r) = try_visualization(
        &file,
        &format,
        json_output,
        critical_resources,
        layers,
        &prune,
        &highlight,
        stats,
        orphans,
        cluster,
        reverse,
        critical_path,
        &affected,
        depth,
    ) {
        return r;
    }
    cmd_graph(&file, &format, machine.as_deref(), group.as_deref())
}
