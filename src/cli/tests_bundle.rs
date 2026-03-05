//! Tests: FJ-1406 self-contained recipe bundles.

#![allow(unused_imports)]
use super::bundle::*;
use super::commands::*;
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
    fn test_fj1406_bundle_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: bundle-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#,
        );
        cmd_bundle(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1406_bundle_with_store() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: store-bundle\nmachines: {}\nresources: {}\n",
        );
        // Create a store directory with a file
        let store_dir = dir.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();
        std::fs::write(store_dir.join("artifact.tar.gz"), b"fake archive").unwrap();
        cmd_bundle(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1406_bundle_with_state() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: state-bundle\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(state_dir.join("m1")).unwrap();
        std::fs::write(
            state_dir.join("m1").join("state.lock.yaml"),
            "resources: {}\nlast_apply: \"2024-01-01\"\n",
        )
        .unwrap();
        cmd_bundle(&file, None, true).unwrap();
    }

    #[test]
    fn test_fj1406_bundle_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        cmd_bundle(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1406_bundle_with_output_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: output\nmachines: {}\nresources: {}\n",
        );
        let out = dir.path().join("bundle.tar");
        cmd_bundle(&file, Some(&out), false).unwrap();
    }

    #[test]
    fn test_fj1406_bundle_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        dispatch(
            Commands::Bundle(BundleArgs {
                file,
                output: None,
                include_state: false,
                verify: false,
            }),
            0,
            true,
        )
        .unwrap();
    }
}
