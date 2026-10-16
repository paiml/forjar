//! forjar#614: the nightly's Windows leg must build vendored OpenSSL with the
//! Strawberry perl the image ships, never Git's msys perl.
//!
//! # The defect
//!
//! `nightly.yml`'s "Build release binary" step runs under `shell: bash`, and
//! on `windows-latest` that resolves Git's msys perl. It lacks core modules
//! (`Params::Check`), so openssl-src's `./Configure` died. The Windows leg was
//! red, the all-or-nothing `release` job was skipped, and the `nightly` tag sat
//! 11 days behind main while every other leg built.
//!
//! # Why this EXECUTES the step instead of reading it
//!
//! What matters is which perl the OpenSSL build is handed, and what happens
//! when that perl is unusable. The test lifts the parsed step's `run:` out of
//! the workflow, substitutes the two expressions GitHub would, and runs it
//! under the shell GitHub uses for `shell: bash`. `cargo` and `cross` are
//! stubs that record the `OPENSSL_SRC_PERL` they were started with. The
//! Strawberry path `C:/Strawberry/perl/bin/perl.exe` has no leading `/`, so on
//! Linux it resolves relative to the step's working directory: each test plants
//! (or withholds) a stub perl there. A comment in the workflow cannot satisfy
//! any of this; only the script's behaviour can.
//!
//! # mutations — each measured to redden its own tests
//!
//! 1. Delete the `if [ "${{ runner.os }}" = Windows ]` block → the Windows leg
//!    reaches cargo with no `OPENSSL_SRC_PERL`, and with an unusable perl it
//!    still builds: `windows_openssl_is_built_with_strawberry_perl` and
//!    `windows_without_a_usable_strawberry_perl_fails_before_cargo` go RED.
//! 2. Drop the `|| { ...; exit 1; }` probe failure → an unusable perl is
//!    handed to the build anyway:
//!    `windows_without_a_usable_strawberry_perl_fails_before_cargo` goes RED.
//! 3. Make the block unconditional → the Linux and aarch64 legs die looking
//!    for a Windows perl: `non_windows_legs_leave_openssl_src_perl_unset` goes
//!    RED.

use serde_yaml_ng::Value;
use std::path::Path;
use std::process::{Command, Output};

const STRAWBERRY: &str = "C:/Strawberry/perl/bin/perl.exe";

/// The build step's script, with the expressions GitHub would have substituted.
fn build_script(runner_os: &str, target: &str) -> String {
    let path = format!(
        "{}/.github/workflows/nightly.yml",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let wf: Value = serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"));
    let step = wf
        .get("jobs")
        .and_then(|j| j.get("build"))
        .and_then(|j| j.get("steps"))
        .and_then(Value::as_sequence)
        .expect("nightly.yml has no jobs.build.steps")
        .iter()
        .find(|s| s.get("name").and_then(Value::as_str) == Some("Build release binary"))
        .expect("jobs.build has no step named `Build release binary`");
    assert_eq!(
        step.get("shell").and_then(Value::as_str),
        Some("bash"),
        "the build step's shell changed; this test models `shell: bash`"
    );
    step.get("run")
        .and_then(Value::as_str)
        .expect("build step has no run:")
        .replace("${{ runner.os }}", runner_os)
        .replace("${{ matrix.target }}", target)
}

/// A stub that records `OPENSSL_SRC_PERL` (or `unset`) into `<cwd>/<name>.called`.
fn recorder(bin: &Path, name: &str) {
    let p = bin.join(name);
    std::fs::write(
        &p,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"${{OPENSSL_SRC_PERL-unset}}\" > \"$PWD/{name}.called\"\n"
        ),
    )
    .expect("write stub");
    chmod_x(&p);
}

fn chmod_x(p: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

/// Plant the Strawberry perl in `dir`: exits `rc`, recording its arguments.
fn strawberry(dir: &Path, rc: i32) {
    let p = dir.join(STRAWBERRY);
    std::fs::create_dir_all(p.parent().expect("perl has a parent")).expect("mkdir perl");
    std::fs::write(
        &p,
        format!("#!/bin/sh\nprintf '%s\\n' \"$*\" > \"$PWD/perl.args\"\nexit {rc}\n"),
    )
    .expect("write perl stub");
    chmod_x(&p);
}

/// Run the step in `dir` the way GitHub runs `shell: bash`.
fn run(dir: &Path, runner_os: &str, target: &str) -> Output {
    let bin = dir.join("stub-bin");
    std::fs::create_dir_all(&bin).expect("mkdir stub-bin");
    recorder(&bin, "cargo");
    recorder(&bin, "cross");
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new("bash")
        .args([
            "--noprofile",
            "--norc",
            "-eo",
            "pipefail",
            "-c",
            &build_script(runner_os, target),
        ])
        .current_dir(dir)
        .env("PATH", path)
        .env_remove("OPENSSL_SRC_PERL")
        .output()
        .expect("spawn bash")
}

fn called(dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(format!("{name}.called")))
        .ok()
        .map(|s| s.trim().to_string())
}

#[test]
fn windows_openssl_is_built_with_strawberry_perl() {
    let dir = tempfile::tempdir().expect("tempdir");
    strawberry(dir.path(), 0);
    let out = run(dir.path(), "Windows", "x86_64-pc-windows-msvc");
    assert!(
        out.status.success(),
        "Windows leg failed with a usable Strawberry perl: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        called(dir.path(), "cargo").as_deref(),
        Some(STRAWBERRY),
        "cargo must build OpenSSL with the Strawberry perl (forjar#614)"
    );
    let args = std::fs::read_to_string(dir.path().join("perl.args")).unwrap_or_default();
    assert!(
        args.contains("-MParams::Check"),
        "the perl must be probed for the module msys perl lacked, got args {args:?}"
    );
}

#[test]
fn windows_without_a_usable_strawberry_perl_fails_before_cargo() {
    // A perl that cannot load Params::Check, and no perl at all.
    for planted in [Some(2), None] {
        let dir = tempfile::tempdir().expect("tempdir");
        if let Some(rc) = planted {
            strawberry(dir.path(), rc);
        }
        let out = run(dir.path(), "Windows", "x86_64-pc-windows-msvc");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !out.status.success(),
            "planted={planted:?}: the step must fail loudly, it exited 0: {stdout}"
        );
        assert!(
            stdout.contains("::error::no usable Strawberry perl"),
            "planted={planted:?}: the failure must name the perl, got: {stdout}"
        );
        assert_eq!(
            called(dir.path(), "cargo"),
            None,
            "planted={planted:?}: cargo must not run with an unusable perl"
        );
    }
}

#[test]
fn non_windows_legs_leave_openssl_src_perl_unset() {
    for (os, target, tool) in [
        ("Linux", "x86_64-unknown-linux-gnu", "cargo"),
        ("Linux", "aarch64-unknown-linux-gnu", "cross"),
        ("macOS", "aarch64-apple-darwin", "cargo"),
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = run(dir.path(), os, target);
        assert!(
            out.status.success(),
            "{os}/{target} leg failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            called(dir.path(), tool).as_deref(),
            Some("unset"),
            "{os}/{target}: {tool} must build with the system perl, OPENSSL_SRC_PERL unset"
        );
    }
}
