//! forjar#893 — the installed control-plane binary must prove where it came
//! from and must be able to do the job BD-3 needs (`secrets keygen`).
//!
//! 2026-11-06: lambda-labs ran a forjar built from a /tmp feature worktree with
//! `default` features only. `--version` said `1.33.0` and nothing else, and
//! `secrets keygen` failed with "compiled without encryption support".

use std::process::Command;

fn forjar(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(args)
        .output()
        .expect("run forjar")
}

fn head_sha() -> Option<String> {
    let root = env!("CARGO_MANIFEST_DIR");
    if !std::path::Path::new(root).join(".git").exists() {
        return None;
    }
    let o = Command::new("git")
        .args(["-C", root, "rev-parse", "HEAD"])
        .output()
        .ok()?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

#[test]
fn version_carries_the_build_sha_or_says_unknown() {
    let o = forjar(&["--version"]);
    assert!(o.status.success(), "--version failed");
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    match head_sha() {
        Some(sha) => assert!(
            out.contains(&sha[..8]),
            "--version lacks the HEAD sha {} in {out:?}",
            &sha[..8]
        ),
        None => assert!(
            out.contains("sha unknown"),
            "no git checkout: --version must say `sha unknown`, got {out:?}"
        ),
    }
}

#[test]
fn version_names_the_sha_field_explicitly() {
    let out = String::from_utf8_lossy(&forjar(&["--version"]).stdout).into_owned();
    assert!(
        out.contains("sha "),
        "--version has no `sha` field: {out:?}"
    );
}

#[test]
fn secrets_keygen_exits_zero_in_the_shipped_feature_set() {
    let o = forjar(&["secrets", "keygen"]);
    assert!(
        o.status.success(),
        "secrets keygen failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("AGE-SECRET-KEY-") || out.contains("age1"),
        "no key material: {out}"
    );
}
