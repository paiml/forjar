//! Coverage tests for cli/graph_scoring.rs — impact, stability, fanout, weight, bottleneck, clustering, cycle risk.

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, yaml.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();
    f
}

const CONFIG: &str = "version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n  my-config:\n    type: file\n    path: /etc/app.conf\n    content: hello\n    requires:\n      - nginx\n  my-service:\n    type: service\n    service_name: nginx\n    requires:\n      - my-config\n";

// ── cmd_graph_resource_impact_score ──

#[test]
fn graph_impact_score_text() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_impact_score(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn graph_impact_score_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_impact_score(cfg.path(), true);
    assert!(r.is_ok());
}

// ── cmd_graph_resource_stability_score ──

#[test]
fn graph_stability_score_text() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_stability_score(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn graph_stability_score_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_stability_score(cfg.path(), true);
    assert!(r.is_ok());
}

// ── cmd_graph_resource_dependency_fanout ──

#[test]
fn graph_fanout_text() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_dependency_fanout(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn graph_fanout_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_dependency_fanout(cfg.path(), true);
    assert!(r.is_ok());
}

// ── cmd_graph_resource_dependency_weight ──

#[test]
fn graph_weight_text() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_dependency_weight(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn graph_weight_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_dependency_weight(cfg.path(), true);
    assert!(r.is_ok());
}

// ── cmd_graph_resource_dependency_bottleneck ──

#[test]
fn graph_bottleneck_text() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_dependency_bottleneck(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn graph_bottleneck_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_dependency_bottleneck(cfg.path(), true);
    assert!(r.is_ok());
}

// ── cmd_graph_resource_type_clustering ──

#[test]
fn graph_type_clustering_text() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_type_clustering(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn graph_type_clustering_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_type_clustering(cfg.path(), true);
    assert!(r.is_ok());
}

// ── cmd_graph_resource_dependency_cycle_risk ──

#[test]
fn graph_cycle_risk_text() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_dependency_cycle_risk(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn graph_cycle_risk_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::graph_scoring::cmd_graph_resource_dependency_cycle_risk(cfg.path(), true);
    assert!(r.is_ok());
}

// ── missing config ──

#[test]
fn graph_impact_score_missing() {
    let r = super::graph_scoring::cmd_graph_resource_impact_score(
        std::path::Path::new("/nonexistent/f.yaml"), false,
    );
    assert!(r.is_err());
}
