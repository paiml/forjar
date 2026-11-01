//! Tests for the `fb_sweepable` shell guard emitted by the prelude.

use super::*;

/// Run the real `fb_sweepable` from the emitted prelude under `sh`.
fn sweepable(cand: &str, prefixes: &[&str]) -> bool {
    let script = format!("fb_log() {{ :; }}\n{}\nfb_sweepable \"$@\"", prelude());
    std::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .arg("sh")
        .arg(cand)
        .args(prefixes)
        .status()
        .expect("sh runs")
        .success()
}

#[test]
fn sweepable_accepts_a_direct_child_of_tmp_under_a_tmp_root() {
    // forjar#627: a rule rooted at `/tmp/*-target` matches depth-2 paths,
    // and the old depth >= 3 floor refused every one of them, so the
    // rule validated, ran, and reclaimed nothing.
    let dir = std::path::Path::new("/tmp").join(format!("fj627-{}-target", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.to_str().unwrap().to_string();
    let ok = sweepable(&p, &["/tmp"]);
    std::fs::remove_dir_all(&dir).ok();
    assert!(ok, "{p} under declared prefix /tmp must be sweepable");
}

#[test]
fn sweepable_refuses_a_candidate_outside_every_declared_root() {
    // The comment in the reaper claimed this re-check existed; it did not.
    let here = env!("CARGO_MANIFEST_DIR");
    assert!(!sweepable(here, &["/tmp"]), "{here} is not under /tmp");
    assert!(!sweepable(here, &[]), "no declared root means no deletion");
}

#[test]
fn sweepable_refuses_the_root_itself_and_top_level_dirs() {
    assert!(
        !sweepable("/tmp", &["/tmp"]),
        "a root is not its own descendant"
    );
    assert!(!sweepable("/home", &["/"]), "`/` is never a usable prefix");
    let home = std::env::var("HOME").unwrap();
    assert!(!sweepable(&home, &["/home"]), "$HOME itself is never swept");
}
