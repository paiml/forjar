//! Tests: Coverage for graph_intelligence_ext, graph_scoring, graph_export, graph_advanced.

use super::graph_advanced::*;
use super::graph_export::*;
use super::graph_intelligence_ext::*;
use super::graph_scoring::*;
use std::io::Write;

const EMPTY_CFG: &str = "version: \"1.0\"\nname: empty\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n";

const DEPS_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: service\n    machine: m\n    name: nginx\n    depends_on: [b]\n";

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(yaml.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// graph_intelligence_ext — cmd_graph_resource_dependency_fan_out

#[test]
fn fan_out_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_fan_out(f.path(), false).is_ok());
}

#[test]
fn fan_out_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_fan_out(f.path(), false).is_ok());
}

#[test]
fn fan_out_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_fan_out(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_intelligence_ext — cmd_graph_resource_dependency_fan_in
// ---------------------------------------------------------------------------

#[test]
fn fan_in_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_fan_in(f.path(), false).is_ok());
}

#[test]
fn fan_in_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_fan_in(f.path(), false).is_ok());
}

#[test]
fn fan_in_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_fan_in(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_intelligence_ext — cmd_graph_resource_dependency_path_count
// ---------------------------------------------------------------------------

#[test]
fn path_count_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_path_count(f.path(), false).is_ok());
}

#[test]
fn path_count_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_path_count(f.path(), false).is_ok());
}

#[test]
fn path_count_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_path_count(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_intelligence_ext — cmd_graph_resource_dependency_articulation_points
// ---------------------------------------------------------------------------

#[test]
fn articulation_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_articulation_points(f.path(), false).is_ok());
}

#[test]
fn articulation_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_articulation_points(f.path(), false).is_ok());
}

#[test]
fn articulation_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_articulation_points(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_intelligence_ext — cmd_graph_resource_dependency_longest_path
// ---------------------------------------------------------------------------

#[test]
fn longest_path_ext_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_longest_path(f.path(), false).is_ok());
}

#[test]
fn longest_path_ext_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_longest_path(f.path(), false).is_ok());
}

#[test]
fn longest_path_ext_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_longest_path(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_intelligence_ext — cmd_graph_resource_dependency_strongly_connected
// ---------------------------------------------------------------------------

#[test]
fn strongly_connected_ext_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_strongly_connected(f.path(), false).is_ok());
}

#[test]
fn strongly_connected_ext_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_strongly_connected(f.path(), false).is_ok());
}

#[test]
fn strongly_connected_ext_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_strongly_connected(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_intelligence_ext — cmd_graph_resource_dependency_topological_depth
// ---------------------------------------------------------------------------

#[test]
fn topo_depth_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_topological_depth(f.path(), false).is_ok());
}

#[test]
fn topo_depth_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_topological_depth(f.path(), false).is_ok());
}

#[test]
fn topo_depth_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_topological_depth(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_intelligence_ext — cmd_graph_resource_dependency_weak_links
// ---------------------------------------------------------------------------

#[test]
fn weak_links_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_weak_links(f.path(), false).is_ok());
}

#[test]
fn weak_links_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_weak_links(f.path(), false).is_ok());
}

#[test]
fn weak_links_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_weak_links(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_intelligence_ext — cmd_graph_resource_dependency_minimum_cut
// ---------------------------------------------------------------------------

#[test]
fn minimum_cut_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_minimum_cut(f.path(), false).is_ok());
}

#[test]
fn minimum_cut_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_minimum_cut(f.path(), false).is_ok());
}

#[test]
fn minimum_cut_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_minimum_cut(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_intelligence_ext — cmd_graph_resource_dependency_dominator_tree
// ---------------------------------------------------------------------------

#[test]
fn dominator_tree_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_dominator_tree(f.path(), false).is_ok());
}

#[test]
fn dominator_tree_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_dominator_tree(f.path(), false).is_ok());
}

#[test]
fn dominator_tree_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_dominator_tree(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_impact_score
// ---------------------------------------------------------------------------

#[test]
fn impact_score_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_impact_score(f.path(), false).is_ok());
}

#[test]
fn impact_score_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_impact_score(f.path(), false).is_ok());
}

#[test]
fn impact_score_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_impact_score(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_stability_score
// ---------------------------------------------------------------------------

#[test]
fn stability_score_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_stability_score(f.path(), false).is_ok());
}

#[test]
fn stability_score_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_stability_score(f.path(), false).is_ok());
}

#[test]
fn stability_score_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_stability_score(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_dependency_fanout
// ---------------------------------------------------------------------------

#[test]
fn scoring_fanout_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_fanout(f.path(), false).is_ok());
}

#[test]
fn scoring_fanout_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_fanout(f.path(), false).is_ok());
}

#[test]
fn scoring_fanout_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_fanout(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_dependency_weight
// ---------------------------------------------------------------------------

#[test]
fn dep_weight_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_weight(f.path(), false).is_ok());
}

#[test]
fn dep_weight_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_weight(f.path(), false).is_ok());
}

#[test]
fn dep_weight_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_weight(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_dependency_bottleneck
// ---------------------------------------------------------------------------

#[test]
fn bottleneck_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_bottleneck(f.path(), false).is_ok());
}

#[test]
fn bottleneck_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_bottleneck(f.path(), false).is_ok());
}

#[test]
fn bottleneck_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_bottleneck(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_type_clustering
// ---------------------------------------------------------------------------

#[test]
fn type_clustering_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_type_clustering(f.path(), false).is_ok());
}

#[test]
fn type_clustering_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_type_clustering(f.path(), false).is_ok());
}

#[test]
fn type_clustering_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_type_clustering(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_dependency_cycle_risk
// ---------------------------------------------------------------------------

#[test]
fn cycle_risk_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_cycle_risk(f.path(), false).is_ok());
}

#[test]
fn cycle_risk_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_cycle_risk(f.path(), false).is_ok());
}

#[test]
fn cycle_risk_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_cycle_risk(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_impact_radius_analysis
// ---------------------------------------------------------------------------

#[test]
fn impact_radius_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_impact_radius_analysis(f.path(), false).is_ok());
}

#[test]
fn impact_radius_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_impact_radius_analysis(f.path(), false).is_ok());
}

#[test]
fn impact_radius_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_impact_radius_analysis(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_dependency_health_map
// ---------------------------------------------------------------------------

#[test]
fn health_map_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_health_map(f.path(), false).is_ok());
}

#[test]
fn health_map_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_health_map(f.path(), false).is_ok());
}

#[test]
fn health_map_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_health_map(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_change_propagation
// ---------------------------------------------------------------------------

#[test]
fn change_propagation_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_change_propagation(f.path(), false).is_ok());
}

#[test]
fn change_propagation_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_change_propagation(f.path(), false).is_ok());
}

#[test]
fn change_propagation_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_change_propagation(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_dependency_depth_analysis
// ---------------------------------------------------------------------------

#[test]
fn depth_analysis_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_depth_analysis(f.path(), false).is_ok());
}

#[test]
fn depth_analysis_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_depth_analysis(f.path(), false).is_ok());
}

#[test]
fn depth_analysis_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_depth_analysis(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_dependency_fan_analysis
// ---------------------------------------------------------------------------

#[test]
fn fan_analysis_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_fan_analysis(f.path(), false).is_ok());
}

#[test]
fn fan_analysis_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_fan_analysis(f.path(), false).is_ok());
}

#[test]
fn fan_analysis_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_fan_analysis(f.path(), true).is_ok());
}
