//! FJ-036: shared harness for the disk-budget falsification tests.
//!
//! These tests EXECUTE the generated shell against a real directory tree and
//! assert on what is left on disk afterwards. They exist because the failure
//! this resource replaces was invisible to exactly the kind of test that greps
//! generated text: the old reaper's script was *correct-looking* and shipped
//! green for months while reclaiming nothing.
//!
//! `df` is stubbed through PATH so a test can drive the watermark logic to any
//! pressure level deterministically. Everything else — `find`, `du`, `stat`,
//! `git`, `rm` — is the real thing, so what is under test is the actual
//! detection and deletion behaviour, not a description of it.

#![allow(dead_code)]

use forjar::core::types::{MachineTarget, ReclaimRule, Resource, ResourceType};
use forjar::resources::disk_budget;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Unique temp dir (no external tempfile dep in this test target).
pub fn tmpdir(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "forjar-budget-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&base).unwrap();
    base
}

/// A cargo target dir in the shape a REPO ROOT actually has: `.rustc_info.json`
/// and, on the measured fleet, no `CACHEDIR.TAG`.
pub fn mk_cargo_target(p: &Path) {
    fs::create_dir_all(p.join("debug")).unwrap();
    fs::write(p.join(".rustc_info.json"), "{}").unwrap();
    fs::write(p.join("debug/blob"), vec![0u8; 4096]).unwrap();
}

/// The other real shape: a per-arch subdirectory, which carries `CACHEDIR.TAG`
/// and no `.rustc_info.json`. Requiring both markers made these invisible.
pub fn mk_cargo_target_arch(p: &Path) {
    fs::create_dir_all(p.join("release")).unwrap();
    fs::write(p.join("CACHEDIR.TAG"), "Signature: 8a477f597d28d172").unwrap();
    fs::write(p.join("release/blob"), vec![0u8; 4096]).unwrap();
}

/// A cargo REGISTRY: `CACHEDIR.TAG`, no `.rustc_info.json`, and children that
/// are source/cache/index rather than build output. Must never be reclaimed.
pub fn mk_cargo_registry(p: &Path) {
    fs::create_dir_all(p.join("src")).unwrap();
    fs::create_dir_all(p.join("cache")).unwrap();
    fs::write(p.join("CACHEDIR.TAG"), "Signature: 8a477f597d28d172").unwrap();
    fs::write(p.join("src/lib.rs"), "// vendored source").unwrap();
}

/// Install a `df` shim that pops one "used_pct free_kb" line per invocation.
pub fn install_df_stub(bin: &Path, sequence: &[(u32, u64)]) {
    fs::create_dir_all(bin).unwrap();
    let state = bin.join("df.state");
    let mut s = String::new();
    for (used, free_kb) in sequence {
        s.push_str(&format!("{used} {free_kb}\n"));
    }
    fs::write(&state, s).unwrap();
    let shim = format!(
        "#!/bin/sh\n\
         S={state:?}\n\
         L=$(head -1 \"$S\")\n\
         if [ \"$(wc -l <\"$S\")\" -gt 1 ]; then tail -n +2 \"$S\" >\"$S.tmp\" && mv \"$S.tmp\" \"$S\"; fi\n\
         set -- $L\n\
         echo 'Filesystem 1024-blocks Used Available Capacity Mounted on'\n\
         echo \"/dev/stub 1000000000 1 $2 $1% /\"\n"
    );
    let p = bin.join("df");
    fs::write(&p, shim).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
}

pub fn budget_resource(path: &Path, rules: Vec<ReclaimRule>) -> Resource {
    Resource {
        resource_type: ResourceType::DiskBudget,
        machine: MachineTarget::Single("test".into()),
        path: Some(path.to_string_lossy().into_owned()),
        budget_high_watermark_pct: Some(85),
        budget_target_free_pct: Some(20),
        budget_reclaim: rules,
        ..Default::default()
    }
}

/// Extract the reaper body from the apply script and run it directly.
pub struct RunResult {
    pub code: i32,
    pub stdout: String,
}

/// Pull the reaper body out of the apply script's heredoc.
pub fn reaper_body(res: &Resource) -> String {
    const OPEN: &str = "<<'FORJAR_REAPER_EOF'\n";
    const CLOSE: &str = "\nFORJAR_REAPER_EOF\n";
    let apply = disk_budget::apply_script(res);
    let start = apply.find(OPEN).expect("reaper heredoc open") + OPEN.len();
    let end = apply[start..].find(CLOSE).expect("reaper heredoc close") + start;
    apply[start..end].to_string()
}

pub fn run_reaper(res: &Resource, bin: &Path, work: &Path) -> RunResult {
    let body = reaper_body(res);

    let script = work.join("reaper.sh");
    fs::write(&script, &body).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let path_env = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("/bin/sh")
        .arg(&script)
        .env("PATH", path_env)
        .env("FORJAR_BUDGET_STATUS", work.join("status.json"))
        .output()
        .expect("run reaper");
    RunResult {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
    }
}

/// Backdate everything under `p` so the idle floor is satisfied.
pub fn backdate(p: &Path) {
    let _ = Command::new("find")
        .args([
            p.to_str().unwrap(),
            "-exec",
            "touch",
            "-d",
            "3 days ago",
            "{}",
            "+",
        ])
        .output();
}
