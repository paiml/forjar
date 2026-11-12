//! Running one resource check on one machine, and what its output may show.
//!
//! Split out of `check.rs` (forjar#996). Since the check resolves
//! `{{secrets.*}}` through the manifest provider, a check script carries live
//! secret values, and a failing script can echo them. The detail a check keeps
//! is therefore redacted before it is printed or serialized, with the policy
//! `apply` uses for its transcripts.

use super::check::{make_check_result, CheckResult};
use crate::core::{resolver, types};
use crate::transport;

/// What a check's captured output may show for one resource.
pub(super) struct Redaction {
    /// Secret values the declaration names; struck out of the detail.
    secrets: Vec<String>,
    /// The values cannot be named (`sensitive`, or ciphertext decrypted after
    /// template substitution), so the detail is withheld whole.
    suppress: bool,
}

impl Redaction {
    /// The policy for one resource, read off the UNRESOLVED declaration: the
    /// resolved one can no longer say which bytes came from `{{secrets.*}}`.
    pub(super) fn for_resource(resource: &types::Resource, secrets: &types::SecretsConfig) -> Self {
        Self {
            secrets: resolver::collect_secret_values(resource, secrets),
            suppress: resource.sensitive || resolver::carries_ciphertext(resource),
        }
    }

    /// The form of `text` that may leave the process.
    pub(super) fn apply(&self, text: &str) -> String {
        if self.suppress && !text.trim().is_empty() {
            return "[withheld: sensitive resource]".to_string();
        }
        resolver::redact_transcript(text, &self.secrets)
    }
}

/// Ensure container is running if needed.
fn ensure_container_if_needed(machine: &types::Machine) -> Result<(), String> {
    if machine.is_container_transport() {
        transport::container::ensure_container(machine)?;
    }
    Ok(())
}

/// Print check failure in text mode.
fn print_check_failure(resource_id: &str, machine_name: &str, detail: &str) {
    println!("  FAIL {resource_id} ({machine_name}) — {detail}");
}

/// Run a check script on one machine and record the result.
pub(super) fn run_single_check(
    machine: &types::Machine,
    check_script: &str,
    resource_id: &str,
    machine_name: &str,
    json: bool,
    redaction: &Redaction,
) -> (CheckResult, bool) {
    if let Err(e) = ensure_container_if_needed(machine) {
        if !json {
            print_check_failure(resource_id, machine_name, &e);
        }
        return (
            make_check_result(resource_id, machine_name, "error", None, e),
            false,
        );
    }

    let output = transport::exec_script(machine, check_script);
    let (status, exit_code, detail, passed) = match output {
        Ok(out) if out.success() => ("pass", Some(0), String::new(), true),
        // FJ-2720: exit 2 = NOT APPLICABLE on this host (no systemd, no docker).
        // Neither a pass nor a failure — forjar cannot observe the state, and
        // claiming either would be a guess.
        Ok(out) if out.exit_code == 2 => ("skip", Some(2), out.stdout.trim().to_string(), true),
        Ok(out) => (
            "fail",
            Some(out.exit_code),
            out.stderr.trim().to_string(),
            false,
        ),
        Err(e) => ("error", None, e, false),
    };
    // forjar#996: redacted BEFORE the text print and the JSON row are cut from it.
    let detail = redaction.apply(&detail);

    if !json {
        if passed {
            println!("  ok {resource_id} ({machine_name})");
        } else {
            let msg = if let Some(code) = exit_code {
                format!("exit {code}")
            } else {
                detail.clone()
            };
            print_check_failure(resource_id, machine_name, &msg);
            if exit_code.is_some() {
                for line in detail.lines().filter(|l| !l.is_empty()) {
                    println!("       {line}");
                }
            }
        }
    }

    (
        make_check_result(resource_id, machine_name, status, exit_code, detail),
        passed,
    )
}
