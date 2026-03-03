//! Tests: FJ-1410 data freshness monitoring.

#![allow(unused_imports)]
use super::commands::*;
use super::data_freshness::*;
use super::dispatch::*;
use super::helpers::*;
use super::test_fixtures::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
        let file = dir.join("forjar.yaml");
        std::fs::write(&file, yaml).unwrap();
        file
    }

    #[test]
    fn test_fj1410_freshness_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: fresh\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        // No artifacts = all fresh
        cmd_data_freshness(&file, &state_dir, Some(24), false).unwrap();
    }

    #[test]
    fn test_fj1410_freshness_with_store() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: store-fresh\nmachines: {}\nresources: {}\n",
        );
        let store_dir = dir.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();
        std::fs::write(store_dir.join("data.bin"), b"fresh data").unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        // Just-created files should be fresh
        cmd_data_freshness(&file, &state_dir, Some(24), false).unwrap();
    }

    #[test]
    fn test_fj1410_freshness_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: json\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_data_freshness(&file, &state_dir, None, true).unwrap();
    }

    #[test]
    fn test_fj1410_freshness_with_state_lock() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: state\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(state_dir.join("forjar.lock.yaml"), "resources: {}\n").unwrap();
        cmd_data_freshness(&file, &state_dir, Some(24), false).unwrap();
    }

    #[test]
    fn test_fj1410_freshness_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        dispatch(
            Commands::DataFreshness(DataFreshnessArgs {
                file,
                state_dir,
                max_age: Some(24),
                json: false,
            }),
            false,
            true,
        )
        .unwrap();
    }
}
