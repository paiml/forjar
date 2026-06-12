//! Tests: Coverage for dispatch_infra_cmd in dispatch_misc_b.rs, driven
//! through dispatch_data_cmd's fall-through arm (PMAT-088).

use super::commands::*;
use super::dispatch_misc_b::dispatch_data_cmd;
use std::path::{Path, PathBuf};

/// Single localhost machine + one file resource under /etc (privileged path
/// so fault-inject emits a permission-denied scenario).
fn write_cfg(dir: &Path) -> PathBuf {
    let yaml = "version: \"1.0\"\nname: infra\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /etc/forjar-cov.conf\n    content: x\n";
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infra_fault_inject_text_ok() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let r = dispatch_data_cmd(Commands::FaultInject(FaultInjectArgs {
            file,
            resource: None,
            json: false,
        }));
        assert!(r.is_ok(), "fault inject report: {r:?}");
    }

    #[test]
    fn infra_fault_inject_json_filtered_ok() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let r = dispatch_data_cmd(Commands::FaultInject(FaultInjectArgs {
            file,
            resource: Some("a".to_string()),
            json: true,
        }));
        assert!(r.is_ok());
    }

    #[test]
    fn infra_fault_inject_bad_config_err() {
        let r = dispatch_data_cmd(Commands::FaultInject(FaultInjectArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            resource: None,
            json: false,
        }));
        assert!(r.is_err());
    }

    #[test]
    fn infra_invariants_text_ok() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let r = dispatch_data_cmd(Commands::Invariants(InvariantsArgs {
            file,
            state_dir: dir.path().join("state"),
            json: false,
        }));
        assert!(r.is_ok(), "invariants: {r:?}");
    }

    #[test]
    fn infra_invariants_json_ok() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let r = dispatch_data_cmd(Commands::Invariants(InvariantsArgs {
            file,
            state_dir: dir.path().join("state"),
            json: true,
        }));
        assert!(r.is_ok());
    }

    #[test]
    fn infra_iso_export_ok() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let out = dir.path().join("iso");
        let r = dispatch_data_cmd(Commands::IsoExport(IsoExportArgs {
            file,
            state_dir: dir.path().join("state"),
            output: out.clone(),
            include_binary: false,
            json: false,
        }));
        assert!(r.is_ok(), "iso export: {r:?}");
        assert!(out.join("config").join("forjar.yaml").exists());
    }

    #[test]
    fn infra_iso_export_json_ok() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let r = dispatch_data_cmd(Commands::IsoExport(IsoExportArgs {
            file,
            state_dir: dir.path().join("state"),
            output: dir.path().join("iso2"),
            include_binary: false,
            json: true,
        }));
        assert!(r.is_ok());
    }

    #[test]
    fn infra_import_brownfield_no_matching_types_ok() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("imported.yaml");
        // "bogus" scan type parses to nothing → no discovery, still writes config.
        let r = dispatch_data_cmd(Commands::ImportBrownfield(ImportBrownfieldArgs {
            machine: "localhost".to_string(),
            scan_types: vec!["bogus".to_string()],
            output: Some(out.clone()),
            json: true,
        }));
        assert!(r.is_ok(), "brownfield import: {r:?}");
        assert!(out.exists(), "generated config should be written");
    }

    #[test]
    fn infra_cross_deps_text_ok() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let r = dispatch_data_cmd(Commands::CrossDeps(CrossDepsArgs { file, json: false }));
        assert!(r.is_ok(), "cross deps: {r:?}");
    }

    #[test]
    fn infra_cross_deps_json_ok() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let r = dispatch_data_cmd(Commands::CrossDeps(CrossDepsArgs { file, json: true }));
        assert!(r.is_ok());
    }

    fn image_args(file: PathBuf, dir: &Path) -> ImageArgs {
        ImageArgs {
            file,
            machine: None,
            user_data: true,
            android: false,
            base: None,
            output: Some(dir.join("user-data.yaml")),
            disk: "auto-lvm".to_string(),
            locale: "en_US.UTF-8".to_string(),
            timezone: "Etc/UTC".to_string(),
            json: false,
        }
    }

    #[test]
    fn infra_image_user_data_ok() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let args = image_args(file, dir.path());
        let r = dispatch_data_cmd(Commands::Image(args));
        assert!(r.is_ok(), "user-data image: {r:?}");
        assert!(dir.path().join("user-data.yaml").exists());
    }

    #[test]
    fn infra_image_android_ok() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let mut args = image_args(file, dir.path());
        args.user_data = false;
        args.android = true;
        args.output = Some(dir.path().join("module.zip"));
        let r = dispatch_data_cmd(Commands::Image(args));
        assert!(r.is_ok(), "android magisk module: {r:?}");
        assert!(dir.path().join("module.zip").exists());
    }

    #[test]
    fn infra_image_base_iso_missing_err() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        let mut args = image_args(file, dir.path());
        args.user_data = false;
        args.base = Some(PathBuf::from("/nonexistent/base.iso"));
        args.output = Some(dir.path().join("out.iso"));
        let r = dispatch_data_cmd(Commands::Image(args));
        assert!(r.is_err(), "missing base ISO must fail");
    }

    #[test]
    fn infra_fall_through_to_platform_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_cfg(dir.path());
        // Not an infra command → routed to dispatch_platform_cmd.
        let r = dispatch_data_cmd(Commands::StackGraph(StackGraphArgs {
            file: vec![file],
            json: true,
        }));
        // Outcome depends on platform handler; the arm itself is what we cover.
        let _ = r;
    }
}
