//! Tests: Core graph commands.

#![allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::graph_core::*;
use super::graph_analysis::*;
use super::graph_cross::*;
use super::graph_topology::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj674_graph_machine_groups() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m1\n    path: /tmp/a\n  b:\n    type: file\n    machine: m2\n    path: /tmp/b\n").unwrap();
        let result = cmd_graph_machine_groups(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj674_graph_machine_groups_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n").unwrap();
        let result = cmd_graph_machine_groups(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj684_graph_resource_clusters() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    depends_on: [a]\n").unwrap();
        let result = cmd_graph_resource_clusters(&f, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj694_graph_fan_out() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    depends_on: [a]\n").unwrap();
        let result = cmd_graph_fan_out(&f, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj704_graph_leaf_resources() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    depends_on: [a]\n").unwrap();
        let result = cmd_graph_leaf_resources(&f, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj714_graph_reverse_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    depends_on: [a]\n").unwrap();
        let result = cmd_graph_reverse_deps(&f, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj714_graph_reverse_deps_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n").unwrap();
        let result = cmd_graph_reverse_deps(&f, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj724_graph_depth_first() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    depends_on: [a]\n").unwrap();
        let result = cmd_graph_depth_first(&f, false);
        assert!(result.is_ok());
    }

}
