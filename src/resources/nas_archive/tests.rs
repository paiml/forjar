//! Tests for `nas_archive`.
//!
//! The important ones RUN the generated script against real directories. This
//! is the one resource on the fleet whose output deletes data, and a test that
//! only greps the generated text proves the string contains `rm` — not that the
//! guard before it holds. `forjar check` passed everything for five months
//! while dozens of `script.contains(...)` tests were green.

use super::*;
use crate::core::types::{Resource, ResourceType};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn resource(path: &str, dest: &str, dirs: &[&str]) -> Resource {
    let mut r = Resource {
        resource_type: ResourceType::NasArchive,
        path: Some(path.to_string()),
        ..Default::default()
    };
    r.archive.destination = Some(dest.to_string());
    r.archive.dirs = dirs.iter().map(|s| s.to_string()).collect();
    r.archive.min_age_days = Some(0); // fixtures are new by construction
    r
}

/// A scratch tree that cleans up after itself.
struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "forjar-nas-archive-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn join(&self, s: &str) -> PathBuf {
        self.0.join(s)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(p: &Path, body: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

/// Run a generated archive script with `bash`, returning (code, stdout+stderr).
fn run(script: &str, execute: bool) -> (i32, String) {
    let dir = std::env::temp_dir().join(format!(
        "forjar-arch-run-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    fs::create_dir_all(&dir).unwrap();
    let f = dir.join("archive.sh");
    fs::write(&f, script).unwrap();
    let out = Command::new("bash")
        .arg(&f)
        .env("ARCHIVE_EXECUTE", if execute { "1" } else { "0" })
        .output()
        .expect("bash");
    let _ = fs::remove_dir_all(&dir);
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

// ── the generated script must survive forjar's own I8 gate ──────────────────

/// forjar refuses to execute a script bashrs rejects, so a resource whose own
/// output fails that gate is permanently red — which is how the lambda-labs
/// audio resource cascaded to eight dependents.
#[test]
fn the_generated_script_passes_forjars_own_i8_gate() {
    let a = archive_of(&resource("/mnt/nvme-raid0", "/mnt/unas/media", &["corpus"])).unwrap();
    let script = archive_script(&a);
    if let Err(e) = crate::core::purifier::validate_script(&script) {
        panic!("nas_archive generates a script forjar itself rejects:\n{e}");
    }
}

#[test]
fn the_check_and_apply_scripts_pass_the_i8_gate() {
    let r = resource(
        "/mnt/nvme-raid0",
        "/mnt/unas/media",
        &["corpus", "albor-data"],
    );
    for (name, s) in [
        ("check", check_script(&r)),
        ("apply", apply_script(&r)),
        ("state_query", state_query_script(&r)),
    ] {
        if let Err(e) = crate::core::purifier::validate_script(&s) {
            panic!("nas_archive {name}_script fails forjar's I8 gate:\n{s}\n\n{e}");
        }
    }
}

// ── behaviour: it moves, and it proves before it deletes ────────────────────

#[test]
fn a_dry_run_moves_nothing() {
    let s = Scratch::new("dry");
    let (src, dest) = (s.join("src"), s.join("dest"));
    write(&src.join("corpus/a.bin"), &"x".repeat(200_000));
    fs::create_dir_all(&dest).unwrap();

    let a = archive_of(&resource(
        src.to_str().unwrap(),
        dest.to_str().unwrap(),
        &["corpus"],
    ))
    .unwrap();
    let (code, out) = run(&archive_script(&a), false);

    assert_eq!(code, 0, "{out}");
    assert!(out.contains("DRY-RUN would archive corpus"), "{out}");
    assert!(src.join("corpus/a.bin").exists(), "dry run moved data");
    assert!(
        !dest.join("corpus").exists(),
        "dry run wrote the destination"
    );
}

#[test]
fn an_archived_directory_is_verified_then_replaced_by_a_symlink() {
    let s = Scratch::new("move");
    let (src, dest) = (s.join("src"), s.join("dest"));
    write(&src.join("corpus/a.bin"), &"a".repeat(200_000));
    write(&src.join("corpus/nested/b.bin"), &"b".repeat(200_000));
    fs::create_dir_all(&dest).unwrap();

    let a = archive_of(&resource(
        src.to_str().unwrap(),
        dest.to_str().unwrap(),
        &["corpus"],
    ))
    .unwrap();
    let (code, out) = run(&archive_script(&a), true);

    assert_eq!(code, 0, "{out}");
    assert!(out.contains("VERIFIED corpus"), "{out}");
    assert!(out.contains("archived=1"), "{out}");
    // Data is at the destination, byte for byte.
    assert_eq!(
        fs::read(dest.join("corpus/a.bin")).unwrap().len(),
        200_000,
        "destination content is wrong"
    );
    assert!(dest.join("corpus/nested/b.bin").exists());
    // The old location is a symlink to the new one.
    let meta = fs::symlink_metadata(src.join("corpus")).unwrap();
    assert!(meta.file_type().is_symlink(), "no symlink left behind");
    assert_eq!(
        fs::canonicalize(src.join("corpus")).unwrap(),
        fs::canonicalize(dest.join("corpus")).unwrap()
    );
}

#[test]
fn a_second_pass_is_a_no_op() {
    let s = Scratch::new("idem");
    let (src, dest) = (s.join("src"), s.join("dest"));
    write(&src.join("corpus/a.bin"), &"a".repeat(100_000));
    fs::create_dir_all(&dest).unwrap();
    let a = archive_of(&resource(
        src.to_str().unwrap(),
        dest.to_str().unwrap(),
        &["corpus"],
    ))
    .unwrap();

    let (c1, _) = run(&archive_script(&a), true);
    assert_eq!(c1, 0);
    let (c2, out2) = run(&archive_script(&a), true);
    assert_eq!(c2, 0, "{out2}");
    assert!(out2.contains("already a symlink"), "{out2}");
    assert!(out2.contains("archived=0"), "{out2}");
}

/// THE safety property. If the destination does not match the source, the
/// source must survive. This is the defect that mattered most in the
/// predecessor: it printed `verified: 0 files differ` when rsync had FAILED,
/// then deleted the source.
#[test]
fn a_destination_that_does_not_match_is_never_deleted_from() {
    let s = Scratch::new("mismatch");
    let (src, dest) = (s.join("src"), s.join("dest"));
    write(&src.join("corpus/a.bin"), &"a".repeat(100_000));
    // A foreign tree already sits at the destination: same name, extra file.
    write(&dest.join("corpus/intruder.bin"), &"z".repeat(100_000));

    let a = archive_of(&resource(
        src.to_str().unwrap(),
        dest.to_str().unwrap(),
        &["corpus"],
    ))
    .unwrap();
    let (code, out) = run(&archive_script(&a), true);

    assert_ne!(code, 0, "a mismatched destination did not fail:\n{out}");
    assert!(out.contains("VERIFY FAILED"), "{out}");
    assert!(
        src.join("corpus/a.bin").exists(),
        "THE SOURCE WAS DELETED after a failed verify:\n{out}"
    );
}

#[test]
fn a_small_file_tree_is_refused_rather_than_moved() {
    let s = Scratch::new("small");
    let (src, dest) = (s.join("src"), s.join("dest"));
    // 600 tiny files: over SMALL_FILE_PCT_MIN_FILES and 100% under 64 KiB.
    for i in 0..600 {
        write(&src.join(format!("corpus/f{i}.txt")), "tiny");
    }
    fs::create_dir_all(&dest).unwrap();

    let mut r = resource(src.to_str().unwrap(), dest.to_str().unwrap(), &["corpus"]);
    r.archive.max_files = Some(10_000); // not the file-count guard
    let a = archive_of(&r).unwrap();
    let (code, out) = run(&archive_script(&a), true);

    assert_eq!(code, 0, "{out}");
    assert!(out.contains("under 65536B"), "{out}");
    assert!(src.join("corpus/f0.txt").exists(), "refused tree was moved");
}

/// The percentage must NOT fire on a small directory: `entrenar-checkpoints`
/// was 8 files, 87% under 64 KiB, and an ordinary archive target.
#[test]
fn a_handful_of_small_files_is_not_refused() {
    let s = Scratch::new("fewsmall");
    let (src, dest) = (s.join("src"), s.join("dest"));
    for i in 0..7 {
        write(&src.join(format!("cp/s{i}.txt")), "tiny");
    }
    write(&src.join("cp/big.bin"), &"b".repeat(200_000));
    fs::create_dir_all(&dest).unwrap();

    let a = archive_of(&resource(
        src.to_str().unwrap(),
        dest.to_str().unwrap(),
        &["cp"],
    ))
    .unwrap();
    let (code, out) = run(&archive_script(&a), true);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("VERIFIED cp"),
        "a small directory was refused:\n{out}"
    );
}

#[test]
fn a_directory_over_the_file_ceiling_is_refused() {
    let s = Scratch::new("many");
    let (src, dest) = (s.join("src"), s.join("dest"));
    for i in 0..12 {
        write(&src.join(format!("corpus/f{i}.bin")), &"x".repeat(70_000));
    }
    fs::create_dir_all(&dest).unwrap();

    let mut r = resource(src.to_str().unwrap(), dest.to_str().unwrap(), &["corpus"]);
    r.archive.max_files = Some(5);
    let a = archive_of(&r).unwrap();
    let (code, out) = run(&archive_script(&a), true);

    assert_eq!(code, 0, "{out}");
    assert!(out.contains("exceeds archive_max_files"), "{out}");
    assert!(src.join("corpus/f0.bin").exists());
}

#[test]
fn an_unwritable_destination_stops_before_touching_anything() {
    let s = Scratch::new("nodest");
    let src = s.join("src");
    write(&src.join("corpus/a.bin"), &"a".repeat(100_000));

    let a = archive_of(&resource(
        src.to_str().unwrap(),
        s.join("does-not-exist").to_str().unwrap(),
        &["corpus"],
    ))
    .unwrap();
    let (code, out) = run(&archive_script(&a), true);

    assert_ne!(code, 0, "{out}");
    assert!(out.contains("does not exist"), "{out}");
    assert!(src.join("corpus/a.bin").exists());
}

// ── declaration-level ───────────────────────────────────────────────────────

#[test]
fn state_query_names_every_declared_directory() {
    // The whole point: a directory that was never archived must be VISIBLE,
    // not silently absent from a policy string nobody diffs.
    let q = state_query_script(&resource(
        "/mnt/nvme-raid0",
        "/mnt/unas/media",
        &["corpus", "albor-data", "gemma2-models"],
    ));
    for d in ["corpus", "albor-data", "gemma2-models"] {
        assert!(q.contains(d), "state query omits {d}:\n{q}");
    }
}

#[test]
fn a_bad_declaration_is_refused_by_every_entry_point() {
    // Destination inside the source: the copy-onto-itself shape.
    let r = resource("/mnt/raid", "/mnt/raid/archive", &["a"]);
    for s in [check_script(&r), apply_script(&r), state_query_script(&r)] {
        assert!(
            s.contains("ERROR:"),
            "a bad declaration produced a live script:\n{s}"
        );
        assert!(s.contains("exit 1"), "{s}");
    }
}

#[test]
fn the_check_script_reports_each_directory_separately() {
    let c = check_script(&resource(
        "/mnt/nvme-raid0",
        "/mnt/unas/media",
        &["corpus", "albor-data"],
    ));
    assert!(
        c.contains("archived:corpus") || c.contains("'archived:corpus'"),
        "{c}"
    );
    assert!(c.contains("archive-pending:albor-data"), "{c}");
}
