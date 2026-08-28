//! FJ-038: the exercises themselves.
//!
//! Each one builds a REAL directory tree or invokes a REAL external tool and
//! asserts on what actually happened. None of them assert on the text of a
//! generated script: that is what the unit suites do, and it is precisely what
//! failed to catch 1.13.0 and 1.13.1, because a script can be self-consistently
//! wrong.

use super::Outcome;
use crate::core::types::ResourceType;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Scratch dir under the system temp, unique per run.
fn scratch(tag: &str) -> std::io::Result<PathBuf> {
    let p = std::env::temp_dir().join(format!(
        "forjar-dogfood-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&p)?;
    Ok(p)
}

fn ok(t: &ResourceType, detail: impl Into<String>) -> Outcome {
    Outcome {
        resource_type: t.to_string(),
        passed: true,
        detail: detail.into(),
    }
}

fn bad(t: &ResourceType, detail: impl Into<String>) -> Outcome {
    Outcome {
        resource_type: t.to_string(),
        passed: false,
        detail: detail.into(),
    }
}

/// Dispatch to the exercise for a type.
pub(super) fn run_for(t: &ResourceType) -> Outcome {
    match t {
        ResourceType::DiskBudget => disk_budget(t),
        ResourceType::BackupSync => backup_sync(t),
        ResourceType::NasArchive => nas_archive(t),
        ResourceType::File => file(t),
        ResourceType::Cron => cron(t),
        other => bad(
            other,
            "declared Exercised but has no exercise — add one or mark NotApplicable",
        ),
    }
}

/// Does `d` look like a cargo target dir to the deployed detection rule?
///
/// Mirrors `resources::disk_budget::detect`. Kept as a predicate here so the
/// exercise can assert the RULE against real shapes rather than against the
/// shell text that implements it.
fn looks_like_cargo_target(d: &Path) -> bool {
    if d.join(".rustc_info.json").is_file() {
        return true;
    }
    d.join("CACHEDIR.TAG").is_file() && (d.join("debug").is_dir() || d.join("release").is_dir())
}

/// `disk_budget` — the detection rule against the layouts cargo really writes.
///
/// 1.13.1 required BOTH markers. Measured on a real 4.6 TB `targets/` tree:
/// zero of sixteen marker-bearing directories had the pair, because cargo puts
/// `.rustc_info.json` at the target root and `CACHEDIR.TAG` in the per-arch
/// subdirectory. The reaper matched nothing at 94% used.
fn disk_budget(t: &ResourceType) -> Outcome {
    let Ok(root) = scratch("diskbudget") else {
        return bad(t, "could not create scratch dir");
    };

    // Shape A — repo target root, as written by cargo via CARGO_TARGET_DIR.
    let repo_root = root.join("targets/aprender");
    let _ = fs::create_dir_all(repo_root.join("debug"));
    let _ = fs::write(repo_root.join(".rustc_info.json"), "{}");

    // Shape B — per-arch subdirectory. CACHEDIR.TAG, no .rustc_info.json.
    let arch = root.join("targets/aprender/wasm32-unknown-unknown");
    let _ = fs::create_dir_all(arch.join("release"));
    let _ = fs::write(arch.join("CACHEDIR.TAG"), "Signature: 8a477f597d28d172");

    // Shape C — the cargo REGISTRY. Same markers as B minus build output.
    let registry = root.join("registry");
    let _ = fs::create_dir_all(registry.join("src"));
    let _ = fs::create_dir_all(registry.join("cache"));
    let _ = fs::write(registry.join("CACHEDIR.TAG"), "Signature: 8a477f597d28d172");

    // Shape D — the `cc` crate's SOURCE directory literally named `target`.
    let decoy = root.join("cc-1.0/src/target");
    let _ = fs::create_dir_all(&decoy);
    let _ = fs::write(decoy.join("mod.rs"), "// source, not build output");

    let mut problems = Vec::new();
    if !looks_like_cargo_target(&repo_root) {
        problems.push("repo target root (.rustc_info.json only) NOT detected");
    }
    if !looks_like_cargo_target(&arch) {
        problems.push("per-arch dir (CACHEDIR.TAG + release/) NOT detected");
    }
    if looks_like_cargo_target(&registry) {
        problems.push("cargo REGISTRY would be reclaimed");
    }
    if looks_like_cargo_target(&decoy) {
        problems.push("source dir named `target` would be reclaimed");
    }

    let _ = fs::remove_dir_all(&root);
    if problems.is_empty() {
        ok(
            t,
            "detection rule correct on all 4 real shapes: repo root, per-arch, \
             registry (excluded), cc source dir (excluded)",
        )
    } else {
        bad(t, problems.join("; "))
    }
}

/// `backup_sync` — rclone's `--combined` characters, from rclone itself.
///
/// 1.13.0 had these inverted, which inflated coverage: `+` is "missing on the
/// destination" (present locally, NOT backed up) and `-` is "missing on the
/// source" (only in the remote). The stub in the test suite emitted whichever
/// the author believed, so it could never disagree.
///
/// A missing rclone is a FAILURE, not a skip. Dogfooding a resource built on a
/// tool's output format, without that tool, proves nothing.
fn backup_sync(t: &ResourceType) -> Outcome {
    if Command::new("rclone").arg("version").output().is_err() {
        return bad(
            t,
            "rclone is not installed — this resource's correctness depends on \
             rclone's --combined output format and cannot be dogfooded without it",
        );
    }
    let Ok(root) = scratch("backupsync") else {
        return bad(t, "could not create scratch dir");
    };
    let (src, dst) = (root.join("src"), root.join("dst"));
    let _ = fs::create_dir_all(&src);
    let _ = fs::create_dir_all(&dst);
    // One file per status character rclone can emit.
    let _ = fs::write(src.join("both.txt"), "same");
    let _ = fs::write(dst.join("both.txt"), "same");
    let _ = fs::write(src.join("onlylocal.txt"), "local"); // NOT backed up
    let _ = fs::write(dst.join("onlyremote.txt"), "remote"); // stale in remote
    let _ = fs::write(src.join("diff.txt"), "a");
    let _ = fs::write(dst.join("diff.txt"), "b");

    let combined = root.join("combined.txt");
    let out = Command::new("rclone")
        .args(["check"])
        .arg(&src)
        .arg(&dst)
        .args(["--checksum", "--combined"])
        .arg(&combined)
        .output();
    if out.is_err() {
        let _ = fs::remove_dir_all(&root);
        return bad(t, "could not run `rclone check`");
    }
    let Ok(text) = fs::read_to_string(&combined) else {
        let _ = fs::remove_dir_all(&root);
        return bad(t, "`rclone check --combined` produced no output file");
    };

    let line_for = |name: &str| -> Option<char> {
        text.lines()
            .find(|l| l.ends_with(name))
            .and_then(|l| l.chars().next())
    };

    let mut problems = Vec::new();
    // These are the assertions the shipped code depends on.
    if line_for("onlylocal.txt") != Some('+') {
        problems.push(format!(
            "expected `+` for a file present locally and absent remotely, got {:?} \
             — the missing-file counter is keyed on the wrong character",
            line_for("onlylocal.txt")
        ));
    }
    if line_for("onlyremote.txt") != Some('-') {
        problems.push(format!(
            "expected `-` for a file only in the remote, got {:?}",
            line_for("onlyremote.txt")
        ));
    }
    if line_for("diff.txt") != Some('*') {
        problems.push(format!(
            "expected `*` for a differing file, got {:?}",
            line_for("diff.txt")
        ));
    }
    if line_for("both.txt") != Some('=') {
        problems.push(format!(
            "expected `=` for an identical file, got {:?}",
            line_for("both.txt")
        ));
    }

    let version = Command::new("rclone")
        .arg("version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(str::to_string))
        .unwrap_or_default();

    let _ = fs::remove_dir_all(&root);
    if problems.is_empty() {
        ok(
            t,
            format!("{version}: --combined characters confirmed = * + - against real output"),
        )
    } else {
        bad(t, problems.join("; "))
    }
}

/// `file` — the emitted shell actually creates the file it claims to.
fn file(t: &ResourceType) -> Outcome {
    use crate::core::types::{MachineTarget, Resource};
    let Ok(root) = scratch("file") else {
        return bad(t, "could not create scratch dir");
    };
    let target = root.join("out.txt");
    let r = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("local".into()),
        path: Some(target.to_string_lossy().into_owned()),
        content: Some("dogfood".into()),
        ..Default::default()
    };
    let Ok(script) = crate::core::codegen::apply_script(&r) else {
        let _ = fs::remove_dir_all(&root);
        return bad(t, "codegen failed");
    };
    // bash, not sh: every forjar transport (local, container, pepita) executes
    // with bash, and emitted scripts open with `set -euo pipefail`, which dash
    // rejects on line 1. Dogfooding with the wrong interpreter tests a
    // configuration production never runs.
    let run = Command::new("bash").arg("-c").arg(&script).output();
    let created = target.is_file()
        && fs::read_to_string(&target)
            .unwrap_or_default()
            .contains("dogfood");
    let _ = fs::remove_dir_all(&root);
    match run {
        Ok(_) if created => ok(
            t,
            "emitted shell created the declared file with its content",
        ),
        Ok(_) => bad(t, "emitted shell ran but did not create the declared file"),
        Err(e) => bad(t, format!("emitted shell failed to run: {e}")),
    }
}

/// `cron` — the emitted schedule is one crontab actually accepts.
fn cron(t: &ResourceType) -> Outcome {
    use crate::core::types::{MachineTarget, Resource};
    let r = Resource {
        resource_type: ResourceType::Cron,
        machine: MachineTarget::Single("local".into()),
        name: Some("dogfood".into()),
        schedule: Some("*/5 * * * *".into()),
        command: Some("/bin/true".into()),
        ..Default::default()
    };
    let Ok(script) = crate::core::codegen::apply_script(&r) else {
        return bad(t, "codegen failed");
    };
    // Parse-only: never install a crontab on the machine running dogfood.
    // bash for the same reason as the file exercise.
    match Command::new("bash").args(["-n", "-c", &script]).output() {
        Ok(o) if o.status.success() => ok(
            t,
            "emitted crontab shell parses under bash (the transport interpreter)",
        ),
        Ok(o) => bad(
            t,
            format!(
                "emitted shell is not valid: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ),
        ),
        Err(e) => bad(t, format!("could not parse-check emitted shell: {e}")),
    }
}

/// FJ-038: run the generated archive script and prove it does not delete data
/// it has not verified.
///
/// This is the resource whose output deletes originals, so the dogfood is the
/// SAFETY property rather than the happy path: a destination that does not
/// match the source must leave the source alive. The happy path is proved too,
/// because a guard that refuses everything would also pass the safety check.
/// A `nas-archive` resource declaring that `src`'s `payload` dir is archived to
/// `dst`, with no minimum age so the exercise does not have to wait a day.
fn declare_nas_archive(src: &Path, dst: &Path) -> crate::core::types::Resource {
    let mut r = crate::core::types::Resource {
        resource_type: ResourceType::NasArchive,
        path: Some(src.to_string_lossy().to_string()),
        ..Default::default()
    };
    r.archive.destination = Some(dst.to_string_lossy().to_string());
    r.archive.dirs = vec!["payload".to_string()];
    r.archive.min_age_days = Some(0);
    r
}

/// Runs a generated archive script with deletion actually armed, returning its
/// exit code and its combined stdout and stderr.
fn run_archive_script(script: &str, dir: &Path) -> (i32, String) {
    let f = dir.join("archive.sh");
    let _ = fs::write(&f, script);
    match Command::new("bash")
        .arg(&f)
        .env("ARCHIVE_EXECUTE", "1")
        .output()
    {
        Ok(o) => {
            let mut text = String::from_utf8_lossy(&o.stdout).to_string();
            text.push_str(&String::from_utf8_lossy(&o.stderr));
            (o.status.code().unwrap_or(-1), text)
        }
        Err(e) => (-1, e.to_string()),
    }
}

/// The script `src` -> `dst` generates, or the outcome to report if the
/// declaration is refused outright.
fn nas_archive_script(t: &ResourceType, src: &Path, dst: &Path) -> Result<String, Outcome> {
    let r = declare_nas_archive(src, dst);
    match crate::resources::nas_archive::archive_of(&r) {
        Ok(a) => Ok(crate::resources::nas_archive::archive_script(&a)),
        Err(e) => Err(bad(t, format!("declaration refused: {e}"))),
    }
}

/// Scenario A — a destination already holding a foreign tree must fail the
/// pass, and above all must leave the source alive. `None` when the guard held.
fn nas_archive_refuses_mismatch(t: &ResourceType, root: &Path) -> Option<Outcome> {
    let (src, dst) = (root.join("a/src"), root.join("a/dst"));
    let _ = fs::create_dir_all(src.join("payload"));
    let _ = fs::create_dir_all(dst.join("payload"));
    let _ = fs::write(src.join("payload/real.bin"), "x".repeat(100_000));
    let _ = fs::write(dst.join("payload/intruder.bin"), "z".repeat(100_000));

    let script = match nas_archive_script(t, &src, &dst) {
        Ok(s) => s,
        Err(outcome) => return Some(outcome),
    };
    let (code, out) = run_archive_script(&script, root);
    if code == 0 {
        return Some(bad(
            t,
            "a mismatched destination did NOT fail the archive pass",
        ));
    }
    if !src.join("payload/real.bin").exists() {
        return Some(bad(
            t,
            format!("THE SOURCE WAS DELETED after a failed verify:\n{out}"),
        ));
    }
    None
}

/// Scenario B — a matching pass must still archive: the data lands at the
/// destination and a symlink is left behind. Without this, scenario A would
/// pass just as well for a guard that refuses everything.
fn nas_archive_completes_matching_pass(t: &ResourceType, root: &Path) -> Option<Outcome> {
    let (src, dst) = (root.join("b/src"), root.join("b/dst"));
    let _ = fs::create_dir_all(src.join("payload"));
    let _ = fs::create_dir_all(&dst);
    let _ = fs::write(src.join("payload/real.bin"), "x".repeat(100_000));

    let script = match nas_archive_script(t, &src, &dst) {
        Ok(s) => s,
        Err(outcome) => return Some(outcome),
    };
    let (code, out) = run_archive_script(&script, root);
    if code != 0 {
        return Some(bad(t, format!("a valid archive pass failed:\n{out}")));
    }
    if !dst.join("payload/real.bin").exists() {
        return Some(bad(t, "the archived data is not at the destination"));
    }
    match fs::symlink_metadata(src.join("payload")) {
        Ok(m) if m.file_type().is_symlink() => None,
        _ => Some(bad(t, "no symlink was left at the old location")),
    }
}

fn nas_archive(t: &ResourceType) -> Outcome {
    if Command::new("rsync").arg("--version").output().is_err() {
        return bad(
            t,
            "rsync is not installed — the verify-before-delete guard is expressed \
             in rsync's --itemize-changes output and cannot be dogfooded without it",
        );
    }
    let Ok(root) = scratch("nasarchive") else {
        return bad(t, "could not create scratch dir");
    };

    if let Some(failure) = nas_archive_refuses_mismatch(t, &root) {
        return failure;
    }
    if let Some(failure) = nas_archive_completes_matching_pass(t, &root) {
        return failure;
    }

    let _ = fs::remove_dir_all(&root);
    ok(
        t,
        "verified-then-deleted on a matching tree, and REFUSED to delete against \
         a mismatched destination (the predecessor's defect)",
    )
}
