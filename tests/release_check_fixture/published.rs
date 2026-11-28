//! The published-state helpers: the `cargo`/`curl` stubs and the run of gate R
//! after a tag exists (forjar#1327 split this out of `mod.rs`, the 500-line gate).

use super::{Fixture, Run};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `cargo` and `curl` answering the way a published release makes them answer.
/// Returned as a directory to put FIRST on `PATH`.
pub(crate) fn stub_release_tools(dir: &Path, crate_version: &str, doc_status: &str) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("mkdir bin");
    // `cargo search` prints `name = "x.y.z"    # description`; the script takes
    // field 3 and strips the quotes.
    std::fs::write(
        bin.join("cargo"),
        format!("#!/usr/bin/env bash\nprintf 'forjar = \"{crate_version}\"    # fixture\\n'\n"),
    )
    .expect("write cargo stub");
    std::fs::write(
        bin.join("curl"),
        format!("#!/usr/bin/env bash\nprintf '{{\"doc_status\": {doc_status}}}'\n"),
    )
    .expect("write curl stub");
    for f in ["cargo", "curl"] {
        std::fs::set_permissions(bin.join(f), std::fs::Permissions::from_mode(0o755))
            .expect("chmod");
    }
    bin
}

/// Run the gate with `gh`, `cargo` and `curl` all answering.
pub(crate) fn run_published(fx: &Fixture, gh: &str, bin: &Path) -> Run {
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .arg(fx.root.join("scripts/dogfood/release-check.sh"))
        .current_dir(&fx.root)
        .env("GH", gh)
        .env("PATH", path)
        .env("QUORUM_STORE", &fx.store)
        .output()
        .expect("bash must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}
