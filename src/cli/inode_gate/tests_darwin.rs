//! forjar#995: a macOS target is measured, not refused, through the real
//! transport path (`collect` -> `measure` -> `transport::query`).

use super::*;
use crate::core::inode_pressure::tests_bsd::MAC_DF_I;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn exe(p: &Path, body: &str) {
    std::fs::write(p, body).unwrap();
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A container runtime whose "container" is a Darwin box: it runs the script
/// it is handed with a BSD `uname` and `df` first on PATH.
fn darwin_runtime(dir: &Path) -> std::path::PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    exe(&bin.join("uname"), "#!/bin/sh\necho Darwin\n");
    exe(
        &bin.join("df"),
        &format!(
            "#!/bin/sh\nfor a in \"$@\"; do case \"$a\" in -T) echo 'df: option requires an argument -- T' >&2; \
             exit 64;; esac; done\ncat <<'EOF'\n{MAC_DF_I}EOF\n"
        ),
    );
    let rt = dir.join("fake-runtime");
    exe(
        &rt,
        &format!(
            "#!/bin/sh\ncase \"$1\" in inspect) echo true; exit 0;; esac\nPATH={}:$PATH exec sh\n",
            bin.display()
        ),
    );
    rt
}

#[test]
fn a_darwin_target_is_measured_and_passes_the_apply_pre_check() {
    let d = tempfile::tempdir().unwrap();
    let yaml = format!(
        "version: \"1.0\"\nname: t\nmachines:\n  mini:\n    hostname: mini\n    addr: container\n    \
         transport: container\n    container:\n      runtime: {}\n      image: x\n      ephemeral: false\n\
         resources: {{}}\n",
        darwin_runtime(d.path()).display()
    );
    let c = crate::core::parser::parse_config(&yaml).unwrap();
    let targets = collect(&c, None, false);
    assert!(targets[0].1.is_ok(), "{targets:?}");
    pre_check(&c, &targets, "apply", false).unwrap();
}
