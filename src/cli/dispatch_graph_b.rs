use super::graph_advanced::*;
use super::graph_analytics::*;
use super::graph_analytics_ext::*;
use super::graph_compliance::*;
use super::graph_governance::*;
use super::graph_health::*;
use super::graph_intelligence::*;
use super::graph_intelligence_ext::*;
use super::graph_intelligence_ext2::*;
use super::graph_lifecycle::*;
use super::graph_quality::*;
use super::graph_resilience::*;
use super::graph_resilience_ext::*;
use super::graph_scoring::*;
use super::graph_topology_ext::*;
use super::graph_transport::*;
use super::graph_weight::*;
use std::path::Path;

/// One `graph --resource-*` analysis. Every flag-selected analysis has the same
/// shape: it re-reads the config file and reports its own findings, honouring
/// `--json`.
pub(super) type GraphAnalysis = fn(&Path, bool) -> Result<(), String>;

/// Run the first analysis whose flag is set, in table order, and return its
/// result; `None` when none of the flags in the table is set. Table order is
/// the precedence order, and only the selected analysis is called.
pub(super) fn first_enabled_analysis(
    file: &Path,
    json: bool,
    analyses: &[(bool, GraphAnalysis)],
) -> Option<Result<(), String>> {
    analyses
        .iter()
        .find(|(enabled, _)| *enabled)
        .map(|(_, analysis)| analysis(file, json))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_graph_scoring_inline(
    file: &Path,
    json: bool,
    resource_dependency_bottleneck: bool,
    resource_type_clustering: bool,
    resource_dependency_cycle_risk: bool,
    resource_impact_radius: bool,
    resource_dependency_health_map: bool,
    resource_change_propagation: bool,
    resource_dependency_depth_analysis: bool,
    resource_dependency_fan_analysis: bool,
    resource_dependency_isolation_score: bool,
    resource_dependency_stability_score: bool,
    resource_dependency_critical_path_length: bool,
    resource_dependency_redundancy_score: bool,
) -> Option<Result<(), String>> {
    first_enabled_analysis(
        file,
        json,
        &[
            (
                resource_dependency_bottleneck,
                cmd_graph_resource_dependency_bottleneck,
            ),
            (resource_type_clustering, cmd_graph_resource_type_clustering),
            (
                resource_dependency_cycle_risk,
                cmd_graph_resource_dependency_cycle_risk,
            ),
            (
                resource_impact_radius,
                cmd_graph_resource_impact_radius_analysis,
            ),
            (
                resource_dependency_health_map,
                cmd_graph_resource_dependency_health_map,
            ),
            (
                resource_change_propagation,
                cmd_graph_resource_change_propagation,
            ),
            (
                resource_dependency_depth_analysis,
                cmd_graph_resource_dependency_depth_analysis,
            ),
            (
                resource_dependency_fan_analysis,
                cmd_graph_resource_dependency_fan_analysis,
            ),
            (
                resource_dependency_isolation_score,
                cmd_graph_resource_dependency_isolation_score,
            ),
            (
                resource_dependency_stability_score,
                cmd_graph_resource_dependency_stability_score,
            ),
            (
                resource_dependency_critical_path_length,
                cmd_graph_resource_dependency_critical_path_length,
            ),
            (
                resource_dependency_redundancy_score,
                cmd_graph_resource_dependency_redundancy_score,
            ),
        ],
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_graph_scoring_phase81(
    file: &Path,
    json: bool,
    resource_dependency_centrality_score: bool,
    resource_dependency_bridge_detection: bool,
    resource_dependency_cluster_coefficient: bool,
    resource_dependency_modularity_score: bool,
    resource_dependency_diameter: bool,
    resource_dependency_eccentricity: bool,
    resource_dependency_density: bool,
    resource_dependency_transitivity: bool,
    resource_dependency_fan_out: bool,
    resource_dependency_fan_in: bool,
    resource_dependency_path_count: bool,
    resource_dependency_articulation_points: bool,
) -> Option<Result<(), String>> {
    first_enabled_analysis(
        file,
        json,
        &[
            (
                resource_dependency_centrality_score,
                cmd_graph_resource_dependency_centrality_score,
            ),
            (
                resource_dependency_bridge_detection,
                cmd_graph_resource_dependency_bridge_detection,
            ),
            (
                resource_dependency_cluster_coefficient,
                cmd_graph_resource_dependency_cluster_coefficient,
            ),
            (
                resource_dependency_modularity_score,
                cmd_graph_resource_dependency_modularity_score,
            ),
            (
                resource_dependency_diameter,
                cmd_graph_resource_dependency_diameter,
            ),
            (
                resource_dependency_eccentricity,
                cmd_graph_resource_dependency_eccentricity,
            ),
            (
                resource_dependency_density,
                cmd_graph_resource_dependency_density,
            ),
            (
                resource_dependency_transitivity,
                cmd_graph_resource_dependency_transitivity,
            ),
            (
                resource_dependency_fan_out,
                cmd_graph_resource_dependency_fan_out,
            ),
            (
                resource_dependency_fan_in,
                cmd_graph_resource_dependency_fan_in,
            ),
            (
                resource_dependency_path_count,
                cmd_graph_resource_dependency_path_count,
            ),
            (
                resource_dependency_articulation_points,
                cmd_graph_resource_dependency_articulation_points,
            ),
        ],
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_graph_phase87(
    file: &Path,
    json: bool,
    resource_dependency_longest_path: bool,
    resource_dependency_strongly_connected: bool,
    resource_dependency_topological_depth: bool,
    resource_dependency_weak_links: bool,
    resource_dependency_minimum_cut: bool,
    resource_dependency_dominator_tree: bool,
    resource_dependency_resilience_score: bool,
    resource_dependency_pagerank: bool,
    resource_dependency_betweenness_centrality: bool,
    resource_dependency_closure_size: bool,
    resource_dependency_eccentricity_map: bool,
    resource_dependency_diameter_path: bool,
) -> Option<Result<(), String>> {
    first_enabled_analysis(
        file,
        json,
        &[
            (
                resource_dependency_longest_path,
                cmd_graph_resource_dependency_longest_path,
            ),
            (
                resource_dependency_strongly_connected,
                cmd_graph_resource_dependency_strongly_connected,
            ),
            (
                resource_dependency_topological_depth,
                cmd_graph_resource_dependency_topological_depth,
            ),
            (
                resource_dependency_weak_links,
                cmd_graph_resource_dependency_weak_links,
            ),
            (
                resource_dependency_minimum_cut,
                cmd_graph_resource_dependency_minimum_cut,
            ),
            (
                resource_dependency_dominator_tree,
                cmd_graph_resource_dependency_dominator_tree,
            ),
            (
                resource_dependency_resilience_score,
                cmd_graph_resource_dependency_resilience_score,
            ),
            (
                resource_dependency_pagerank,
                cmd_graph_resource_dependency_pagerank,
            ),
            (
                resource_dependency_betweenness_centrality,
                cmd_graph_resource_dependency_betweenness_centrality,
            ),
            (
                resource_dependency_closure_size,
                cmd_graph_resource_dependency_closure_size,
            ),
            (
                resource_dependency_eccentricity_map,
                cmd_graph_resource_dependency_eccentricity_map,
            ),
            (
                resource_dependency_diameter_path,
                cmd_graph_resource_dependency_diameter_path,
            ),
        ],
    )
}
pub(super) fn try_graph_phase94(
    file: &Path,
    json: bool,
    resource_dependency_bridge_criticality: bool,
    resource_dependency_conditional_subgraph: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_bridge_criticality {
        return Some(cmd_graph_resource_dependency_bridge_criticality(file, json));
    }
    if resource_dependency_conditional_subgraph {
        return Some(cmd_graph_resource_dependency_conditional_subgraph(
            file, json,
        ));
    }
    None
}
pub(super) fn try_graph_phase95(
    file: &Path,
    json: bool,
    resource_dependency_parallel_groups: bool,
    resource_dependency_execution_cost: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_parallel_groups {
        return Some(cmd_graph_resource_dependency_parallel_groups(file, json));
    }
    if resource_dependency_execution_cost {
        return Some(cmd_graph_resource_dependency_execution_cost(file, json));
    }
    None
}
pub(super) fn try_graph_phase96(
    file: &Path,
    json: bool,
    resource_recipe_expansion_map: bool,
    resource_dependency_critical_chain_path: bool,
) -> Option<Result<(), String>> {
    if resource_recipe_expansion_map {
        return Some(cmd_graph_resource_recipe_expansion_map(file, json));
    }
    if resource_dependency_critical_chain_path {
        return Some(cmd_graph_resource_dependency_critical_chain_path(
            file, json,
        ));
    }
    None
}
pub(super) fn try_graph_phase97(
    file: &Path,
    json: bool,
    resource_apply_order_simulation: bool,
    resource_provenance_summary: bool,
) -> Option<Result<(), String>> {
    if resource_apply_order_simulation {
        return Some(cmd_graph_resource_apply_order_simulation(file, json));
    }
    if resource_provenance_summary {
        return Some(cmd_graph_resource_provenance_summary(file, json));
    }
    None
}
pub(super) fn try_graph_phase98(
    file: &Path,
    json: bool,
    resource_dependency_risk_score: bool,
    resource_dependency_layering: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_risk_score {
        return Some(cmd_graph_resource_dependency_risk_score(file, json));
    }
    if resource_dependency_layering {
        return Some(cmd_graph_resource_dependency_layering(file, json));
    }
    None
}
pub(super) fn try_graph_phase99(
    file: &Path,
    json: bool,
    resource_lifecycle_stage_map: bool,
    resource_dependency_age_overlay: bool,
) -> Option<Result<(), String>> {
    if resource_lifecycle_stage_map {
        return Some(cmd_graph_resource_lifecycle_stage_map(file, json));
    }
    if resource_dependency_age_overlay {
        return Some(cmd_graph_resource_dependency_age_overlay(file, json));
    }
    None
}
pub(super) fn try_graph_phase100(
    file: &Path,
    json: bool,
    resource_dependency_health_overlay: bool,
    resource_dependency_width_analysis: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_health_overlay {
        return Some(cmd_graph_resource_dependency_health_overlay(file, json));
    }
    if resource_dependency_width_analysis {
        return Some(cmd_graph_resource_dependency_width_analysis(file, json));
    }
    None
}
pub(super) fn try_graph_phase101(
    file: &Path,
    json: bool,
    resource_dependency_critical_path_highlight: bool,
    resource_dependency_bottleneck_detection: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_critical_path_highlight {
        return Some(cmd_graph_resource_dependency_critical_path_highlight(
            file, json,
        ));
    }
    if resource_dependency_bottleneck_detection {
        return Some(cmd_graph_resource_dependency_bottleneck_detection(
            file, json,
        ));
    }
    None
}
pub(super) fn try_graph_phase102(
    file: &Path,
    json: bool,
    resource_topology_cluster_analysis: bool,
    resource_dependency_island_detection: bool,
) -> Option<Result<(), String>> {
    if resource_topology_cluster_analysis {
        return Some(cmd_graph_resource_topology_cluster_analysis(file, json));
    }
    if resource_dependency_island_detection {
        return Some(cmd_graph_resource_dependency_island_detection(file, json));
    }
    None
}
pub(super) fn try_graph_phase103(
    file: &Path,
    json: bool,
    resource_dependency_depth_histogram_analysis: bool,
    resource_dependency_redundancy_analysis: bool,
) -> Option<Result<(), String>> {
    if resource_dependency_depth_histogram_analysis {
        return Some(cmd_graph_resource_dependency_depth_histogram(file, json));
    }
    if resource_dependency_redundancy_analysis {
        return Some(cmd_graph_resource_dependency_redundancy_analysis(
            file, json,
        ));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_graph_phases_104_106(
    file: &Path,
    json: bool,
    a1: bool,
    a2: bool,
    b1: bool,
    b2: bool,
    c1: bool,
    c2: bool,
) -> Option<Result<(), String>> {
    if a1 {
        return Some(cmd_graph_resource_dependency_change_impact_radius(
            file, json,
        ));
    }
    if a2 {
        return Some(cmd_graph_resource_dependency_sibling_analysis(file, json));
    }
    if b1 {
        return Some(cmd_graph_resource_dependency_fan_in_hotspot(file, json));
    }
    if b2 {
        return Some(cmd_graph_resource_dependency_cross_machine_bridge(
            file, json,
        ));
    }
    if c1 {
        return Some(cmd_graph_resource_dependency_weight_analysis(file, json));
    }
    if c2 {
        return Some(cmd_graph_resource_dependency_topological_summary(
            file, json,
        ));
    }
    None
}
