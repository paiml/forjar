//! FJ-3607: the behavioural half of `forjar dist --verify` — proof that
//! the generated installer RUNS, not merely that it parses.
//!
//! `sh -n` cannot catch a forward call by construction: using a shell
//! function above its definition is valid POSIX syntax that fails only at
//! runtime. That blind spot shipped a published install.sh which exited
//! 127 with `usage: not found` on `--help` while every gate said PASS.
//!
//! So this module executes the script for real, inside a PATH sandbox
//! whose first entry shims every network- and filesystem-mutating tool to
//! refuse. Verifying can never install anything.

use std::path::Path;

/// Executables the installer could use to reach the network or mutate the
/// host. `--verify` shims them to refuse, so executing the script can never
/// install anything even if a future regression falls through to `main`.
const SANDBOX_DENY: [&str; 8] = [
    "curl", "wget", "sudo", "tar", "install", "cp", "chmod", "mktemp",
];

/// One sandboxed `sh <script> <args...>` run.
#[derive(Debug)]
struct ShellRun {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl ShellRun {
    /// `exit N`, or a signal description when the shell died without one.
    fn code_str(&self) -> String {
        match self.code {
            Some(c) => c.to_string(),
            None => "<killed by signal>".to_string(),
        }
    }

    /// First non-blank stderr line — the message a user would actually see.
    fn first_stderr_line(&self) -> String {
        self.stderr
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("<no stderr>")
            .to_string()
    }
}

/// Write one refusing shim and make it executable.
fn write_deny_shim(path: &Path, name: &str) -> Result<(), String> {
    let body =
        format!("#!/bin/sh\necho \"dist --verify sandbox: refusing to run {name}\" >&2\nexit 97\n");
    std::fs::write(path, body).map_err(|e| format!("write {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Build a PATH whose first entry refuses every network/mutation tool.
fn build_deny_path(dir: &Path) -> Result<std::ffi::OsString, String> {
    let deny = dir.join("deny-bin");
    std::fs::create_dir_all(&deny).map_err(|e| format!("cannot create {}: {e}", deny.display()))?;
    for name in SANDBOX_DENY {
        write_deny_shim(&deny.join(name), name)?;
    }
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![deny];
    paths.extend(std::env::split_paths(&inherited));
    std::env::join_paths(paths).map_err(|e| format!("cannot build sandbox PATH: {e}"))
}

/// Run the script under `sh` with the deny PATH and a throwaway `$HOME`.
fn run_sandboxed(
    script: &Path,
    dir: &Path,
    path: &std::ffi::OsStr,
    args: &[&str],
) -> Result<ShellRun, String> {
    let out = std::process::Command::new("sh")
        .arg(script)
        .args(args)
        .current_dir(dir)
        .env("PATH", path)
        .env("HOME", dir)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("cannot spawn sh {}: {e}", script.display()))?;
    Ok(ShellRun {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// `--help` must exit 0 and print the usage block.
fn assert_help_ok(run: &ShellRun) -> Result<(), String> {
    if run.code != Some(0) {
        return Err(format!(
            "`sh install.sh --help` exited {} (want 0): {}",
            run.code_str(),
            run.first_stderr_line()
        ));
    }
    let missing: Vec<&str> = ["USAGE:", "OPTIONS:", "--help"]
        .into_iter()
        .filter(|n| !run.stdout.contains(n))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "`--help` exited 0 but printed no usage (missing: {})",
            missing.join(", ")
        ))
    }
}

/// An unknown flag must reach `die()` — exit 1 with the real message, not
/// a 127 `die: not found`.
fn assert_die_ok(run: &ShellRun) -> Result<(), String> {
    if run.code != Some(1) {
        return Err(format!(
            "`sh install.sh --forjar-verify-bogus` exited {} (want 1): {}",
            run.code_str(),
            run.first_stderr_line()
        ));
    }
    if run.stderr.contains("unknown option") {
        Ok(())
    } else {
        Err(format!(
            "unknown-option path never reached die(): stderr was {:?}",
            run.first_stderr_line()
        ))
    }
}

/// FJ-3607: `sh -n` cannot catch a forward call by construction — using a
/// function before its definition is valid POSIX syntax that only fails at
/// runtime (127, "usage: not found"). So execute the script for real, in a
/// sandbox where every install-capable tool refuses to run.
pub(super) fn check_help_runs(script_path: &Path, dir: &Path) -> Result<(), String> {
    let path = build_deny_path(dir)?;
    assert_help_ok(&run_sandboxed(script_path, dir, &path, &["--help"])?)?;
    assert_die_ok(&run_sandboxed(
        script_path,
        dir,
        &path,
        &["--forjar-verify-bogus"],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("fj-verify-{tag}-{}-{seq}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The exact defect shape: `usage`/`die` called by the argument parser
    /// above their definitions.
    const FORWARD_CALL_INSTALLER: &str = r#"#!/bin/sh
set -eu
while [ $# -gt 0 ]; do
  case "$1" in
    --help|-h) usage; exit 0 ;;
    *) die "unknown option: $1" ;;
  esac
done
die() { printf 'error: %s\n' "$1" >&2; exit 1; }
usage() { echo "USAGE:"; echo "OPTIONS:"; echo "    --help"; }
"#;

    /// The instrument must reject the real defect — and `sh -n` must NOT,
    /// which is precisely why the executing check has to exist.
    #[test]
    fn help_run_rejects_forward_call_that_sh_n_accepts() {
        let dir = scratch("fwd");
        let path = dir.join("install.sh");
        std::fs::write(&path, FORWARD_CALL_INSTALLER).unwrap();

        // Ask the shell itself, not our wrapper: `sh -n` accepts this.
        let syntax = std::process::Command::new("sh")
            .arg("-n")
            .arg(&path)
            .output()
            .expect("spawn sh -n");
        let behaviour = check_help_runs(&path, &dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            syntax.status.success(),
            "a forward call is valid syntax — sh -n cannot catch this: {}",
            String::from_utf8_lossy(&syntax.stderr)
        );
        let err = behaviour.expect_err("executing --help must catch the forward call");
        assert!(err.contains("exited 127"), "got: {err}");
    }

    /// A script whose `--help` exits 0 but prints nothing is still broken.
    #[test]
    fn help_run_rejects_silent_help() {
        let dir = scratch("silent");
        let path = dir.join("install.sh");
        std::fs::write(
            &path,
            "#!/bin/sh\nset -eu\ndie() { echo \"error: $1\" >&2; exit 1; }\n\
             case \"${1:-}\" in --help) exit 0 ;; *) die \"unknown option: $1\" ;; esac\n",
        )
        .unwrap();
        let err = check_help_runs(&path, &dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            err.unwrap_err().contains("printed no usage"),
            "silent --help must fail"
        );
    }

    /// The sandbox must genuinely refuse the install-capable tools — a
    /// check that "executes safely" is worthless if it can still curl.
    #[test]
    fn sandbox_path_refuses_network_and_install_tools() {
        let dir = scratch("sandbox");
        let path = build_deny_path(&dir).unwrap();
        let probe = dir.join("probe.sh");
        std::fs::write(&probe, "#!/bin/sh\ncurl https://example.com\n").unwrap();
        let run = run_sandboxed(&probe, &dir, &path, &[]).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(run.code, Some(97), "curl was not shimmed: {run:?}");
        assert!(
            run.stderr.contains("refusing to run curl"),
            "{}",
            run.stderr
        );
    }
}
