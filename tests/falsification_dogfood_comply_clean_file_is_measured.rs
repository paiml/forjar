//! Gate B: a file bashrs lints CLEAN is measured (0 errors), not unmeasured.
//!
//! `bashrs lint` prints a `Summary:` line only when it has findings; a clean
//! file prints `No issues found in <file>` and exits 0. Gate B once treated
//! the absence of `Summary:` as "did not lint", so the clean
//! `scripts/dogfood/lib/tags.sh` failed the gate. The converse must still
//! hold: output that is neither form (a missing file, a crash) stays
//! UNMEASURED and fails.

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run comply.sh's own `bashrs_count` against a fake `bashrs` printing `out`.
fn run(out: &str) -> (bool, String) {
    let tmp = std::env::temp_dir().join(format!("gateb-{}-{}", std::process::id(), out.len()));
    std::fs::create_dir_all(&tmp).unwrap();
    let fake = tmp.join("bashrs");
    std::fs::write(
        &fake,
        format!("#!/bin/sh\ncat <<'EOF'\n{out}\nEOF\nexit 0\n"),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let script = std::fs::read_to_string(root().join("scripts/dogfood/comply.sh")).unwrap();
    let start = script
        .find("BASHRS_ERR_COUNT=0\n")
        .expect("bashrs_count present");
    let end = script[start..].find("\n}\n").expect("function end") + start + 3;
    let body = format!(
        "fail() {{ echo \"GATE B FAIL $1\"; exit 1; }}\n{}\nbashrs_count x.sh\necho COUNT=$BASHRS_ERR_COUNT\n",
        &script[start..end]
    );
    let o = Command::new("bash")
        .arg("-c")
        .arg(body)
        .env(
            "PATH",
            format!("{}:{}", tmp.display(), std::env::var("PATH").unwrap()),
        )
        .output()
        .unwrap();
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
    )
}

#[test]
fn clean_file_is_zero_errors_not_unmeasured() {
    let (ok, o) = run("No issues found in x.sh");
    assert!(ok, "{o}");
    assert!(o.contains("COUNT=0"), "{o}");
}

#[test]
fn findings_are_still_counted() {
    let (ok, o) = run("x [error] SEC011\nSummary: 1 error(s), 0 warning(s)");
    assert!(ok, "{o}");
    assert!(o.contains("COUNT=1"), "{o}");
}

#[test]
fn unrecognised_output_stays_unmeasured() {
    let (ok, o) = run("no such file");
    assert!(!ok);
    assert!(o.contains("unmeasured is not clean"), "{o}");
}
