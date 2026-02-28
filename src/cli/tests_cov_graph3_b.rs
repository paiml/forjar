//! Tests: Coverage for graph_intelligence_ext, graph_scoring, graph_export, graph_advanced (part 2).

use super::graph_advanced::*;
use super::graph_export::*;
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

// graph_scoring — cmd_graph_resource_dependency_isolation_score

#[test]
fn isolation_score_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_isolation_score(f.path(), false).is_ok());
}

#[test]
fn isolation_score_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_isolation_score(f.path(), false).is_ok());
}

#[test]
fn isolation_score_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_isolation_score(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_scoring — cmd_graph_resource_dependency_stability_score
// ---------------------------------------------------------------------------

#[test]
fn dep_stability_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_stability_score(f.path(), false).is_ok());
}

#[test]
fn dep_stability_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_stability_score(f.path(), false).is_ok());
}

#[test]
fn dep_stability_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_stability_score(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_root_resources
// ---------------------------------------------------------------------------

#[test]
fn root_resources_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_root_resources(f.path(), false).is_ok());
}

#[test]
fn root_resources_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_root_resources(f.path(), false).is_ok());
}

#[test]
fn root_resources_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_root_resources(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_edge_list
// ---------------------------------------------------------------------------

#[test]
fn edge_list_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_edge_list(f.path(), false).is_ok());
}

#[test]
fn edge_list_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_edge_list(f.path(), false).is_ok());
}

#[test]
fn edge_list_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_edge_list(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_connected_components
// ---------------------------------------------------------------------------

#[test]
fn connected_components_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_connected_components(f.path(), false).is_ok());
}

#[test]
fn connected_components_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_connected_components(f.path(), false).is_ok());
}

#[test]
fn connected_components_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_connected_components(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_adjacency_matrix
// ---------------------------------------------------------------------------

#[test]
fn adjacency_matrix_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_adjacency_matrix(f.path(), false).is_ok());
}

#[test]
fn adjacency_matrix_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_adjacency_matrix(f.path(), false).is_ok());
}

#[test]
fn adjacency_matrix_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_adjacency_matrix(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_longest_path
// ---------------------------------------------------------------------------

#[test]
fn longest_path_export_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_longest_path(f.path(), false).is_ok());
}

#[test]
fn longest_path_export_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_longest_path(f.path(), false).is_ok());
}

#[test]
fn longest_path_export_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_longest_path(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_in_degree
// ---------------------------------------------------------------------------

#[test]
fn in_degree_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_in_degree(f.path(), false).is_ok());
}

#[test]
fn in_degree_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_in_degree(f.path(), false).is_ok());
}

#[test]
fn in_degree_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_in_degree(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_out_degree
// ---------------------------------------------------------------------------

#[test]
fn out_degree_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_out_degree(f.path(), false).is_ok());
}

#[test]
fn out_degree_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_out_degree(f.path(), false).is_ok());
}

#[test]
fn out_degree_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_out_degree(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_density
// ---------------------------------------------------------------------------

#[test]
fn density_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_density(f.path(), false).is_ok());
}

#[test]
fn density_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_density(f.path(), false).is_ok());
}

#[test]
fn density_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_density(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_topological_sort
// ---------------------------------------------------------------------------

#[test]
fn topo_sort_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_topological_sort(f.path(), false).is_ok());
}

#[test]
fn topo_sort_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_topological_sort(f.path(), false).is_ok());
}

#[test]
fn topo_sort_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_topological_sort(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_critical_path_resources
// ---------------------------------------------------------------------------

#[test]
fn critical_path_res_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_critical_path_resources(f.path(), false).is_ok());
}

#[test]
fn critical_path_res_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_critical_path_resources(f.path(), false).is_ok());
}

#[test]
fn critical_path_res_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_critical_path_resources(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_export — cmd_graph_sink_resources
// ---------------------------------------------------------------------------

#[test]
fn sink_resources_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_sink_resources(f.path(), false).is_ok());
}

#[test]
fn sink_resources_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_sink_resources(f.path(), false).is_ok());
}

#[test]
fn sink_resources_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_sink_resources(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_advanced — cmd_graph_bipartite_check
// ---------------------------------------------------------------------------

#[test]
fn bipartite_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_bipartite_check(f.path(), false).is_ok());
}

#[test]
fn bipartite_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_bipartite_check(f.path(), false).is_ok());
}

#[test]
fn bipartite_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_bipartite_check(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_advanced — cmd_graph_strongly_connected
// ---------------------------------------------------------------------------

#[test]
fn strongly_connected_adv_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_strongly_connected(f.path(), false).is_ok());
}

#[test]
fn strongly_connected_adv_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_strongly_connected(f.path(), false).is_ok());
}

#[test]
fn strongly_connected_adv_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_strongly_connected(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_advanced — cmd_graph_dependency_matrix_csv
// ---------------------------------------------------------------------------

#[test]
fn dep_matrix_csv_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_dependency_matrix_csv(f.path(), false).is_ok());
}

#[test]
fn dep_matrix_csv_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_dependency_matrix_csv(f.path(), false).is_ok());
}

#[test]
fn dep_matrix_csv_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_dependency_matrix_csv(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_advanced — cmd_graph_resource_weight
// ---------------------------------------------------------------------------

#[test]
fn resource_weight_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_weight(f.path(), false).is_ok());
}

#[test]
fn resource_weight_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_weight(f.path(), false).is_ok());
}

#[test]
fn resource_weight_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_weight(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_advanced — cmd_graph_dependency_depth_per_resource
// ---------------------------------------------------------------------------

#[test]
fn dep_depth_per_res_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_dependency_depth_per_resource(f.path(), false).is_ok());
}

#[test]
fn dep_depth_per_res_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_dependency_depth_per_resource(f.path(), false).is_ok());
}

#[test]
fn dep_depth_per_res_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_dependency_depth_per_resource(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_advanced — cmd_graph_resource_fanin
// ---------------------------------------------------------------------------

#[test]
fn resource_fanin_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_fanin(f.path(), false).is_ok());
}

#[test]
fn resource_fanin_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_fanin(f.path(), false).is_ok());
}

#[test]
fn resource_fanin_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_fanin(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_advanced — cmd_graph_isolated_subgraphs
// ---------------------------------------------------------------------------

#[test]
fn isolated_subgraphs_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_isolated_subgraphs(f.path(), false).is_ok());
}

#[test]
fn isolated_subgraphs_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_isolated_subgraphs(f.path(), false).is_ok());
}

#[test]
fn isolated_subgraphs_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_isolated_subgraphs(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_advanced — cmd_graph_resource_dependency_critical_path_length
// ---------------------------------------------------------------------------

#[test]
fn critical_path_len_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_critical_path_length(f.path(), false).is_ok());
}

#[test]
fn critical_path_len_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_critical_path_length(f.path(), false).is_ok());
}

#[test]
fn critical_path_len_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_critical_path_length(f.path(), true).is_ok());
}

// ---------------------------------------------------------------------------
// graph_advanced — cmd_graph_resource_dependency_redundancy_score
// ---------------------------------------------------------------------------

#[test]
fn redundancy_score_empty_text() {
    let f = write_temp_config(EMPTY_CFG);
    assert!(cmd_graph_resource_dependency_redundancy_score(f.path(), false).is_ok());
}

#[test]
fn redundancy_score_deps_text() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_redundancy_score(f.path(), false).is_ok());
}

#[test]
fn redundancy_score_deps_json() {
    let f = write_temp_config(DEPS_CFG);
    assert!(cmd_graph_resource_dependency_redundancy_score(f.path(), true).is_ok());
}
