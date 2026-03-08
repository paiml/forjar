//! Tests: Infrastructure utilities.

#![allow(unused_imports)]
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::infra::*;
use super::test_fixtures::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj139_cmd_bench_runs() {
        let result = cmd_bench(10, false);
        assert!(result.is_ok(), "bench should succeed: {result:?}");
    }

    #[test]
    fn test_fj139_cmd_bench_json() {
        let result = cmd_bench(10, true);
        assert!(result.is_ok(), "bench JSON should succeed: {result:?}");
    }

    // ── FJ-205: --json output tests ────────────────────────────────

    #[test]
    fn test_fj214_state_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        cmd_state_list(dir.path(), None, false).unwrap();
    }

    #[test]
    fn test_fj214_state_list_empty_json() {
        let dir = tempfile::tempdir().unwrap();
        cmd_state_list(dir.path(), None, true).unwrap();
    }

    #[test]
    fn test_fj214_state_list_nonexistent_dir() {
        cmd_state_list(Path::new("/tmp/nonexistent-state-dir"), None, false).unwrap();
    }

    #[test]
    fn test_fj214_state_list_with_resources() {
        let dir = tempfile::tempdir().unwrap();
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "pkg".to_string(),
            make_test_resource_lock(types::ResourceType::Package),
        );
        resources.insert(
            "cfg".to_string(),
            make_test_resource_lock(types::ResourceType::File),
        );
        let lock = make_test_lock("web01", resources);
        state::save_lock(dir.path(), &lock).unwrap();

        cmd_state_list(dir.path(), None, false).unwrap();
    }

    #[test]
    fn test_fj214_state_list_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "pkg".to_string(),
            make_test_resource_lock(types::ResourceType::Package),
        );
        let lock = make_test_lock("web01", resources);
        state::save_lock(dir.path(), &lock).unwrap();

        cmd_state_list(dir.path(), None, true).unwrap();
    }

    #[test]
    fn test_fj214_state_list_machine_filter() {
        let dir = tempfile::tempdir().unwrap();

        let mut r1 = indexmap::IndexMap::new();
        r1.insert(
            "pkg".to_string(),
            make_test_resource_lock(types::ResourceType::Package),
        );
        state::save_lock(dir.path(), &make_test_lock("web01", r1)).unwrap();

        let mut r2 = indexmap::IndexMap::new();
        r2.insert(
            "db".to_string(),
            make_test_resource_lock(types::ResourceType::Package),
        );
        state::save_lock(dir.path(), &make_test_lock("db01", r2)).unwrap();

        // Filter to web01 only
        cmd_state_list(dir.path(), Some("web01"), false).unwrap();
        // Filter to nonexistent
        cmd_state_list(dir.path(), Some("nonexistent"), false).unwrap();
    }

    // ================================================================
    // FJ-212: state-mv tests
    // ================================================================

    #[test]
    fn test_fj212_state_mv_basic() {
        let dir = tempfile::tempdir().unwrap();
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "old-name".to_string(),
            make_test_resource_lock(types::ResourceType::File),
        );
        state::save_lock(dir.path(), &make_test_lock("web01", resources)).unwrap();

        cmd_state_mv(dir.path(), "old-name", "new-name", None).unwrap();

        let lock = state::load_lock(dir.path(), "web01").unwrap().unwrap();
        assert!(!lock.resources.contains_key("old-name"));
        assert!(lock.resources.contains_key("new-name"));
    }

    #[test]
    fn test_fj212_state_mv_same_id() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_state_mv(dir.path(), "same", "same", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("same"));
    }

    #[test]
    fn test_fj212_state_mv_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let resources = indexmap::IndexMap::new();
        state::save_lock(dir.path(), &make_test_lock("web01", resources)).unwrap();

        let result = cmd_state_mv(dir.path(), "missing", "new", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_fj212_state_mv_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "a".to_string(),
            make_test_resource_lock(types::ResourceType::File),
        );
        resources.insert(
            "b".to_string(),
            make_test_resource_lock(types::ResourceType::File),
        );
        state::save_lock(dir.path(), &make_test_lock("web01", resources)).unwrap();

        let result = cmd_state_mv(dir.path(), "a", "b", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_fj212_state_mv_no_state_dir() {
        let result = cmd_state_mv(Path::new("/tmp/nonexistent-state"), "a", "b", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj212_state_mv_preserves_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let mut resources = indexmap::IndexMap::new();
        let mut res = make_test_resource_lock(types::ResourceType::Package);
        res.hash = "blake3:deadbeef".to_string();
        resources.insert("old-pkg".to_string(), res);
        state::save_lock(dir.path(), &make_test_lock("web01", resources)).unwrap();

        cmd_state_mv(dir.path(), "old-pkg", "new-pkg", None).unwrap();

        let lock = state::load_lock(dir.path(), "web01").unwrap().unwrap();
        assert_eq!(lock.resources["new-pkg"].hash, "blake3:deadbeef");
        assert_eq!(
            lock.resources["new-pkg"].resource_type,
            types::ResourceType::Package
        );
    }

    // ================================================================
    // FJ-213: state-rm tests
    // ================================================================

    #[test]
    fn test_fj213_state_rm_basic() {
        let dir = tempfile::tempdir().unwrap();
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "pkg".to_string(),
            make_test_resource_lock(types::ResourceType::Package),
        );
        resources.insert(
            "cfg".to_string(),
            make_test_resource_lock(types::ResourceType::File),
        );
        state::save_lock(dir.path(), &make_test_lock("web01", resources)).unwrap();

        cmd_state_rm(dir.path(), "pkg", None, false).unwrap();

        let lock = state::load_lock(dir.path(), "web01").unwrap().unwrap();
        assert!(!lock.resources.contains_key("pkg"));
        assert!(lock.resources.contains_key("cfg"));
    }

    #[test]
    fn test_fj213_state_rm_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let resources = indexmap::IndexMap::new();
        state::save_lock(dir.path(), &make_test_lock("web01", resources)).unwrap();

        let result = cmd_state_rm(dir.path(), "missing", None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_fj213_state_rm_force() {
        let dir = tempfile::tempdir().unwrap();
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "pkg".to_string(),
            make_test_resource_lock(types::ResourceType::Package),
        );
        state::save_lock(dir.path(), &make_test_lock("web01", resources)).unwrap();

        cmd_state_rm(dir.path(), "pkg", None, true).unwrap();

        let lock = state::load_lock(dir.path(), "web01").unwrap().unwrap();
        assert!(lock.resources.is_empty());
    }

    #[test]
    fn test_fj213_state_rm_no_state_dir() {
        let result = cmd_state_rm(Path::new("/tmp/nonexistent-state"), "pkg", None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj213_state_rm_machine_filter() {
        let dir = tempfile::tempdir().unwrap();

        let mut r1 = indexmap::IndexMap::new();
        r1.insert(
            "pkg".to_string(),
            make_test_resource_lock(types::ResourceType::Package),
        );
        state::save_lock(dir.path(), &make_test_lock("web01", r1)).unwrap();

        let mut r2 = indexmap::IndexMap::new();
        r2.insert(
            "pkg".to_string(),
            make_test_resource_lock(types::ResourceType::Package),
        );
        state::save_lock(dir.path(), &make_test_lock("db01", r2)).unwrap();

        // Remove only from web01
        cmd_state_rm(dir.path(), "pkg", Some("web01"), false).unwrap();

        let lock_web = state::load_lock(dir.path(), "web01").unwrap().unwrap();
        assert!(lock_web.resources.is_empty());

        let lock_db = state::load_lock(dir.path(), "db01").unwrap().unwrap();
        assert!(lock_db.resources.contains_key("pkg"));
    }

    // ── FJ-2920: OutputWriter adoption for bench ──

    #[test]
    fn test_fj2920_bench_with_null_writer() {
        use crate::cli::output::NullWriter;
        use crate::cli::infra_bench::cmd_bench_with_writer;
        // NullWriter discards all output — benchmark runs but produces no output
        let mut w = NullWriter;
        let result = cmd_bench_with_writer(1, true, &mut w);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj2920_bench_with_test_writer() {
        use crate::cli::output::TestWriter;
        use crate::cli::infra_bench::cmd_bench_with_writer;
        let mut w = TestWriter::new();
        cmd_bench_with_writer(1, true, &mut w).unwrap();
        let json_out = w.stdout_text();
        assert!(
            json_out.contains("\"name\""),
            "JSON bench output: {json_out:?}"
        );
        assert!(
            json_out.contains("\"status\""),
            "should contain status: {json_out:?}"
        );
    }

}
