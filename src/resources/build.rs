//! FJ-33: Build resource handler — cross-compile on one machine, deploy to another.
//!
//! The build resource generates scripts that run on the **deploy machine**.
//! The apply script SSHes to the build machine to compile, then pulls the
//! artifact back to the deploy target with `copia sync`.
//!
//! YAML example:
//! ```yaml
//! apr-binary:
//!   type: build
//!   machine: jetson            # deploy target
//!   build_machine: intel       # where compilation runs
//!   command: "cargo build --release --target aarch64-unknown-linux-gnu -p apr-cli"
//!   working_dir: ~/src/aprender
//!   source: /tmp/cross/release/apr   # artifact path on build machine
//!   target: ~/.cargo/bin/apr         # deploy path on target machine
//!   completion_check: "apr --version"
//! ```

use crate::core::shell_escape::{is_valid_host, sh_squote};
use crate::core::types::Resource;
use crate::resources::verdict;

/// Check if the deployed artifact exists and is executable.
pub fn check_script(resource: &Resource) -> String {
    let deploy_path = resource.target.as_deref().unwrap_or("/dev/null");
    let mut script = verdict::single(
        &format!("test -x {}", sh_squote(deploy_path)),
        "installed:build",
        "missing:build",
    );
    if let Some(ref check) = resource.completion_check {
        script = verdict::single(
            &format!("{check} >/dev/null 2>&1"),
            "installed:build",
            "missing:build",
        );
    }
    script
}

/// Build the remote build command body (cd into working_dir, then run command).
///
/// `command` is intentionally arbitrary shell; `working_dir` is escaped. The
/// body is delivered to the remote over stdin (see `apply_script`), so it is
/// never quote-wrapped as a single argument.
fn build_command_body(resource: &Resource) -> String {
    let command = resource.command.as_deref().unwrap_or("true");
    match resource.working_dir {
        Some(ref dir) => format!("cd {} && {command}", sh_squote(dir)),
        None => command.to_string(),
    }
}

/// Phase 2: get the built artifact onto the deploy machine.
///
/// forjar#290: this shelled out to `scp`, which is not the sovereign tool.
/// `copia sync host:path dest` does exactly this today and needs NOTHING on
/// the remote — it streams over `ssh host "cat …"` — so there was no bootstrap
/// argument for scp, only that it was reached for first.
///
/// The remote spec `host:path` is unchanged: copia's `FileLocation::parse`
/// splits on the first colon just as scp does, so FJ-154's escaping story
/// (validated host + `sh_squote`d artifact path) still holds.
///
/// `chmod +x` is now REQUIRED rather than defensive: copia writes with default
/// permissions.
///
/// The preflight runs BEFORE `mkdir -p`, deliberately. Without it an operator
/// on a machine without copia sees a half-made destination directory and then
/// a message about a missing binary, in that order.
///
/// This is ONE freshly-built binary. copia's remote pull has no delta transfer
/// and buffers the artifact in memory; do not generalise this call to trees.
fn transfer_phase(build_machine: &str, artifact: &str, dp: &str) -> String {
    if build_machine == "localhost" {
        return format!(
            "# Phase 2: copy artifact locally\n\
             mkdir -p \"$(dirname {dp})\"\n\
             cp {} {dp}\n\
             chmod +x {dp}\n",
            sh_squote(artifact)
        );
    }
    let remote_spec = sh_squote(&format!("{build_machine}:{artifact}"));
    format!(
        "# Phase 2: transfer artifact with copia (the sovereign sync tool)\n\
         if ! command -v copia >/dev/null 2>&1; then\n\
         \x20 echo 'ERROR: copia is not installed on this machine - the build resource \
         pulls the artifact with copia sync' >&2\n\
         \x20 echo 'HINT: cargo install copia --features cli' >&2\n\
         \x20 exit 1\n\
         fi\n\
         mkdir -p \"$(dirname {dp})\"\n\
         copia sync {remote_spec} {dp}\n\
         chmod +x {dp}\n"
    )
}

/// Generate the build + transfer + deploy script.
///
/// Runs on the deploy machine. Phases:
/// 1. SSH to build_machine, run build command in working_dir
/// 2. `copia sync` the artifact from build_machine to deploy_path
/// 3. Run completion_check locally
pub fn apply_script(resource: &Resource) -> String {
    let artifact = resource.source.as_deref().unwrap_or("/dev/null");
    let deploy_path = resource.target.as_deref().unwrap_or("/tmp/build-artifact");
    let build_machine = resource.build_machine.as_deref().unwrap_or("localhost");

    // FJ-154: a remote build_machine must be a valid hostname/IP. Reject
    // anything that could inject shell into the ssh/copia argv.
    if build_machine != "localhost" && !is_valid_host(build_machine) {
        return format!(
            "echo {} >&2; exit 1",
            sh_squote(&format!("ERROR: invalid build_machine: {build_machine}"))
        );
    }

    let bm = sh_squote(build_machine);
    let dp = sh_squote(deploy_path);
    let build_cmd = build_command_body(resource);

    let mut script = String::from("set -euo pipefail\n");

    // Phase 1: Build on the build machine.
    if build_machine == "localhost" {
        script.push_str(&format!("# Phase 1: build locally\n{build_cmd}\n"));
    } else {
        // FJ-154: deliver the (arbitrary) build command to the remote over a
        // quoted heredoc piped to `bash -s` on stdin — the codebase's
        // anti-injection pattern — instead of quote-wrapping it as one ssh arg.
        // The quoted `FORJAR_BUILD` delimiter keeps the body literal locally.
        script.push_str(&format!(
            "# Phase 1: build on {build_machine}\n\
             ssh -o BatchMode=yes -o ConnectTimeout=10 {bm} bash -s <<'FORJAR_BUILD'\n\
             {build_cmd}\n\
             FORJAR_BUILD\n"
        ));
    }

    script.push_str(&transfer_phase(build_machine, artifact, &dp));

    // Phase 3: Completion check
    if let Some(ref check) = resource.completion_check {
        script.push_str(&format!("# Phase 3: verify\n{check}\n"));
    }

    script
}

/// State query: hash the deployed artifact for drift detection.
pub fn state_query_script(resource: &Resource) -> String {
    let deploy_path = sh_squote(resource.target.as_deref().unwrap_or("/dev/null"));
    format!(
        "if [ -f {deploy_path} ]; then \
         sha256sum {deploy_path} | awk '{{print $1}}'; \
         else echo 'MISSING'; fi"
    )
}

#[cfg(test)]
mod fj154_tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn build_resource() -> Resource {
        Resource {
            resource_type: ResourceType::Build,
            machine: MachineTarget::Single("jetson".to_string()),
            build_machine: Some("intel".to_string()),
            command: Some("cargo build --release".to_string()),
            working_dir: Some("~/src/aprender".to_string()),
            source: Some("/tmp/cross/release/apr".to_string()),
            target: Some("/home/user/.cargo/bin/apr".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn fj154_remote_build_uses_stdin_heredoc() {
        // Defect #11: the build command is delivered over stdin via a quoted
        // heredoc piped to `bash -s`, not quote-wrapped as a single ssh arg.
        let r = build_resource();
        let script = apply_script(&r);
        assert!(
            script.contains(
                "ssh -o BatchMode=yes -o ConnectTimeout=10 'intel' bash -s <<'FORJAR_BUILD'"
            ),
            "{script}"
        );
        assert!(script.contains("FORJAR_BUILD\n"), "{script}");
        // The old, breakable `ssh ... 'cmd'` quote-wrapping is gone.
        assert!(!script.contains("'intel' 'cd"), "{script}");
    }

    #[test]
    fn fj154_command_with_quotes_survives_in_heredoc() {
        // A build command containing single quotes (e.g. sed 's/a/b/') no
        // longer breaks the remote invocation: it sits literally in the
        // quoted heredoc body.
        let mut r = build_resource();
        r.command = Some("sed 's/a/b/' Cargo.toml".to_string());
        let script = apply_script(&r);
        assert!(script.contains("sed 's/a/b/' Cargo.toml"), "{script}");
        // The body is between the heredoc markers, not inside an ssh arg.
        let after = script.split("<<'FORJAR_BUILD'").nth(1).unwrap_or("");
        assert!(after.contains("sed 's/a/b/'"), "{script}");
    }

    #[test]
    fn fj154_invalid_build_machine_rejected() {
        let mut r = build_resource();
        r.build_machine = Some("intel';reboot;'".to_string());
        let script = apply_script(&r);
        assert!(script.contains("ERROR: invalid build_machine"), "{script}");
        assert!(script.contains("exit 1"), "{script}");
        assert!(!script.contains("ssh"), "{script}");
    }

    #[test]
    fn fj154_copia_remote_spec_quoted() {
        let r = build_resource();
        let script = apply_script(&r);
        assert!(
            script
                .contains("copia sync 'intel:/tmp/cross/release/apr' '/home/user/.cargo/bin/apr'"),
            "{script}"
        );
    }

    #[test]
    fn fj154_working_dir_quote_neutralized() {
        let mut r = build_resource();
        r.build_machine = Some("localhost".to_string());
        r.working_dir = Some("~/x';reboot;'".to_string());
        let script = apply_script(&r);
        assert!(script.contains("'\"'\"'"), "{script}");
        assert!(!script.contains("cd '~/x';reboot"), "{script}");
    }
}
