//! Coverage tests for plan.rs — cmd_plan, cmd_plan_compact, save/load_plan_file.

#![allow(unused_imports)]
use super::plan::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const CFG: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n    state: present\n    depends_on:\n      - pkg\n";

    fn setup_state() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("m1/state.lock.yaml");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "resources:\n  pkg:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 1.0\n").unwrap();
        dir
    }

    #[test]
    fn test_plan_basic() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan(f.path(), d.path(), None, None, None, false, false, None, None, None, false, None, false, &[], None, false);
    }
    #[test]
    fn test_plan_json() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan(f.path(), d.path(), None, None, None, true, false, None, None, None, false, None, false, &[], None, false);
    }
    #[test]
    fn test_plan_verbose() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan(f.path(), d.path(), None, None, None, false, true, None, None, None, false, None, false, &[], None, false);
    }
    #[test]
    fn test_plan_machine_filter() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan(f.path(), d.path(), Some("m1"), None, None, false, false, None, None, None, false, None, false, &[], None, false);
    }
    #[test]
    fn test_plan_no_diff() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan(f.path(), d.path(), None, None, None, false, false, None, None, None, true, None, false, &[], None, false);
    }
    #[test]
    fn test_plan_cost() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan(f.path(), d.path(), None, None, None, false, false, None, None, None, false, None, true, &[], None, false);
    }
    #[test]
    fn test_plan_why() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan(f.path(), d.path(), None, None, None, false, false, None, None, None, false, None, false, &[], None, true);
    }
    #[test]
    fn test_plan_what_if() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let what_if = vec!["pkg=removed".to_string()];
        let _ = cmd_plan(f.path(), d.path(), None, None, None, false, false, None, None, None, false, None, false, &what_if, None, false);
    }
    #[test]
    fn test_plan_target() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan(f.path(), d.path(), None, None, None, false, false, None, None, None, false, Some("pkg"), false, &[], None, false);
    }
    #[test]
    fn test_plan_output_dir() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let out = tempfile::tempdir().unwrap();
        let _ = cmd_plan(f.path(), d.path(), None, None, None, false, false, Some(out.path()), None, None, false, None, false, &[], None, false);
    }
    #[test]
    fn test_plan_plan_out() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let out = tempfile::tempdir().unwrap();
        let plan_file = out.path().join("plan.json");
        let _ = cmd_plan(f.path(), d.path(), None, None, None, false, false, None, None, None, false, None, false, &[], Some(plan_file.as_path()), false);
    }
    #[test]
    fn test_plan_compact() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan_compact(f.path(), d.path(), None, false);
    }
    #[test]
    fn test_plan_compact_json() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan_compact(f.path(), d.path(), None, true);
    }
    #[test]
    fn test_plan_compact_machine() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan_compact(f.path(), d.path(), Some("m1"), false);
    }
    #[test]
    fn test_plan_tag_filter() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan(f.path(), d.path(), None, None, Some("web"), false, false, None, None, None, false, None, false, &[], None, false);
    }
    #[test]
    fn test_plan_resource_filter() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_plan(f.path(), d.path(), None, Some("pkg"), None, false, false, None, None, None, false, None, false, &[], None, false);
    }
}
