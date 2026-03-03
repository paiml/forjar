//! Coverage tests: fleet_ops execution paths + apply_variants dry-run (FJ-1372).

#![allow(unused_imports)]
use super::apply_variants::*;
use super::fleet_ops::*;
use super::helpers::*;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    fn write_cfg(dir: &Path, yaml: &str) -> PathBuf {
        let p = dir.join("forjar.yaml");
        std::fs::write(&p, yaml).unwrap();
        p
    }

    fn local_cfg() -> &'static str {
        r#"version: "1.0"
name: cov-fleet-c
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    user: test
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/fleet-c-test.txt
    content: "hello"
"#
    }

    fn multi_cfg() -> &'static str {
        r#"version: "1.0"
name: cov-multi
machines:
  web-1:
    hostname: web1
    addr: 127.0.0.1
    user: test
  web-2:
    hostname: web2
    addr: 127.0.0.1
    user: test
  web-3:
    hostname: web3
    addr: 127.0.0.1
    user: test
resources:
  f1:
    type: file
    machine: web-1
    path: /tmp/fleet-c-1.txt
    content: "a"
  f2:
    type: file
    machine: web-2
    path: /tmp/fleet-c-2.txt
    content: "b"
  f3:
    type: file
    machine: web-3
    path: /tmp/fleet-c-3.txt
    content: "c"
"#
    }

    fn deps_cfg() -> &'static str {
        r#"version: "1.0"
name: cov-deps
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    user: test
resources:
  base:
    type: file
    machine: local
    path: /tmp/fleet-c-base.txt
    content: "base"
  app:
    type: file
    machine: local
    path: /tmp/fleet-c-app.txt
    content: "app"
    depends_on: [base]
  svc:
    type: file
    machine: local
    path: /tmp/fleet-c-svc.txt
    content: "svc"
    depends_on: [app]
"#
    }

    // ── inventory ────────────────────────────────────────────────

    #[test]
    fn inventory_local_text() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), local_cfg());
        assert!(cmd_inventory(&c, false).is_ok());
    }

    #[test]
    fn inventory_local_json() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), local_cfg());
        assert!(cmd_inventory(&c, true).is_ok());
    }

    #[test]
    fn inventory_multi_text() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), multi_cfg());
        assert!(cmd_inventory(&c, false).is_ok());
    }

    #[test]
    fn inventory_multi_json() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), multi_cfg());
        assert!(cmd_inventory(&c, true).is_ok());
    }

    #[test]
    fn inventory_invalid() {
        assert!(cmd_inventory(Path::new("/nonexistent.yaml"), false).is_err());
    }

    // ── retry_failed ─────────────────────────────────────────────

    #[test]
    fn retry_failed_no_events() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), local_cfg());
        let s = d.path().join("state");
        std::fs::create_dir_all(&s).unwrap();
        assert!(cmd_retry_failed(&c, &s, &[], None).is_ok());
    }

    #[test]
    fn retry_failed_invalid() {
        assert!(cmd_retry_failed(Path::new("/no.yaml"), Path::new("/s"), &[], None).is_err());
    }

    // ── rolling ──────────────────────────────────────────────────

    #[test]
    fn rolling_invalid() {
        assert!(cmd_rolling(Path::new("/no.yaml"), Path::new("/s"), 1, &[], None).is_err());
    }

    // ── canary ───────────────────────────────────────────────────

    #[test]
    fn canary_unknown_machine() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), local_cfg());
        let s = d.path().join("state");
        std::fs::create_dir_all(&s).unwrap();
        let r = cmd_canary(&c, &s, "ghost", false, &[], None);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("not found"));
    }

    #[test]
    fn canary_invalid() {
        let r = cmd_canary(
            Path::new("/no.yaml"),
            Path::new("/s"),
            "x",
            false,
            &[],
            None,
        );
        assert!(r.is_err());
    }

    // ── dry_run_graph ────────────────────────────────────────────

    #[test]
    fn dry_run_graph_with_deps() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), deps_cfg());
        assert!(cmd_apply_dry_run_graph(&c).is_ok());
    }

    #[test]
    fn dry_run_graph_no_deps() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), local_cfg());
        assert!(cmd_apply_dry_run_graph(&c).is_ok());
    }

    #[test]
    fn dry_run_graph_multi() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), multi_cfg());
        assert!(cmd_apply_dry_run_graph(&c).is_ok());
    }

    #[test]
    fn dry_run_graph_invalid() {
        assert!(cmd_apply_dry_run_graph(Path::new("/no.yaml")).is_err());
    }

    // ── dry_run_cost ─────────────────────────────────────────────

    #[test]
    fn dry_run_cost_fresh() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), local_cfg());
        let s = d.path().join("state");
        std::fs::create_dir_all(&s).unwrap();
        assert!(cmd_apply_dry_run_cost(&c, &s, None).is_ok());
    }

    #[test]
    fn dry_run_cost_machine_filter() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), multi_cfg());
        let s = d.path().join("state");
        std::fs::create_dir_all(&s).unwrap();
        assert!(cmd_apply_dry_run_cost(&c, &s, Some("web-1")).is_ok());
    }

    #[test]
    fn dry_run_cost_deps() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), deps_cfg());
        let s = d.path().join("state");
        std::fs::create_dir_all(&s).unwrap();
        assert!(cmd_apply_dry_run_cost(&c, &s, None).is_ok());
    }

    #[test]
    fn dry_run_cost_invalid() {
        assert!(cmd_apply_dry_run_cost(Path::new("/no.yaml"), Path::new("/s"), None).is_err());
    }

    // ── canary_machine variant ───────────────────────────────────

    #[test]
    fn canary_machine_not_found() {
        let d = tempfile::tempdir().unwrap();
        let c = write_cfg(d.path(), local_cfg());
        let s = d.path().join("state");
        std::fs::create_dir_all(&s).unwrap();
        let r = cmd_apply_canary_machine(&c, &s, "ghost", &[], None);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("not found"));
    }

    #[test]
    fn canary_machine_invalid() {
        assert!(
            cmd_apply_canary_machine(Path::new("/no.yaml"), Path::new("/s"), "x", &[], None)
                .is_err()
        );
    }
}
