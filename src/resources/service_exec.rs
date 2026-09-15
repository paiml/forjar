//! PMAT-560: exec parity for a `service` — what the loaded unit EXECUTES.
//!
//! `service.rs` asked the host `is-active` and `is-enabled`. On yoga,
//! 2026-09-15, both were true of `github-runner-ephemeral.service` while the
//! unit systemd had loaded ran `run-ephemeral-docker-v3.sh`, a file the repo
//! never declared. This module adds the two questions that were missing:
//!
//!   * which program does the LOADED unit start — `systemctl show`, so a
//!     drop-in override or an edited unit file is seen for what it is, not
//!     the unit file forjar wrote;
//!   * what are the bytes of THAT program — `sha256sum` over the live path,
//!     never the declared one, because hashing the declared path would
//!     confirm the file forjar just wrote and prove nothing about the unit.
//!
//! Every fragment here is gated on the declaration: a `service` that declares
//! neither `exec_start` nor `exec_sha256` emits none of it, so its check asks
//! exactly what it always asked and its observed digest does not move.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::verdict;

/// Shell variable holding the loaded unit's `ExecStart` program path.
///
/// Named without the word `exec`: bashrs reads `exec` inside an identifier as
/// the builtin and reports SEC016 on every line that mentions it. The fallback
/// groups are `{ …; }`, not `( … )` — a parenthesised group is a subshell to
/// bashrs' SC2031 whether or not the assignment is inside it.
const LIVE: &str = "__fj_unit_prog";
/// Shell variable holding the sha256 of the file at [`LIVE`].
const SHA: &str = "__fj_unit_sha";

/// Read the loaded unit's `ExecStart` program and hash it.
///
/// `systemctl show -p ExecStart --value` prints one
/// `{ path=/x ; argv[]=/x … ; status=0/0 }` line per `ExecStart=`, and an
/// EMPTY line, exit 0, for a unit it has not loaded — so an unloaded unit
/// yields an empty path and `sha256sum ""` yields `missing`, and both diverge
/// from any declaration. A `systemctl show` that FAILS (no dbus, no manager)
/// is folded into the same empty-path shape — under the apply script's
/// `set -eo pipefail` a failing substitution would abort with no marker, and
/// an unmeasured program must read as divergent, never as an opaque death and
/// never as a pass. The awk reads every line and prints the first path: no
/// `head`, so nothing upstream is left writing into a closed pipe under
/// `pipefail` (the SIGPIPE class, PMAT-240). Fields are split on systemd's
/// own ` ; ` separator, not on a bare space — `/opt/my app/run.sh` is a
/// legal program path and a space-split read it as `/opt/my` (found by the
/// PMAT-560 review quorum, 2 of 3 lanes).
pub fn probe(name: &str) -> String {
    let n = sh_squote(name);
    format!(
        "{LIVE}=$( {{ systemctl show -p ExecStart --value {n} 2>/dev/null || echo; }} | \
         awk -F' ; ' '!done && sub(/^\\{{ path=/, \"\", $1) {{ print $1; done=1 }}')\n\
         {SHA}=$( {{ sha256sum \"${LIVE}\" 2>/dev/null || echo missing; }} | awk '{{print $1}}')"
    )
}

/// The divergent branch: the marker, to stdout AND stderr.
///
/// stdout is the `verdict` convention and what the state query is hashed
/// from. stderr is what the operator sees: `cli::check` prints only a failing
/// script's stderr under `exit 1`, and the executor reports a failed apply
/// from stderr too — so a marker on stdout alone names what the unit runs to
/// nobody (found by the PMAT-560 review quorum).
fn diverged(prefix: &str, live_expr: &str) -> String {
    format!(
        "__fj_m={}{live_expr}; echo \"$__fj_m\"; echo \"$__fj_m\" >&2",
        sh_squote(prefix)
    )
}

/// The parity assertions for a check or apply script, in `verdict` form.
///
/// Empty when nothing is declared. Otherwise the probe first, then one
/// assertion per declared field. The divergent marker names the LIVE value,
/// because the operator's next question is "so what IS it running?".
pub fn assertions(resource: &Resource, name: &str) -> Vec<String> {
    if !resource.exec.is_declared() {
        return Vec::new();
    }
    let mut out = vec![probe(name)];
    if let Some(path) = resource.exec.exec_start.as_deref() {
        out.push(verdict::assert_block(
            &format!("[ \"${LIVE}\" = {} ]", sh_squote(path)),
            &format!("echo {}", sh_squote(&format!("exec_start:{name}:{path}"))),
            &diverged(
                &format!("exec_start_diverged:{name}:declared={path}:live="),
                &format!("\"${{{LIVE}:-none}}\""),
            ),
        ));
    }
    if let Some(digest) = resource.exec.exec_sha256.as_deref() {
        out.push(verdict::assert_block(
            &format!("[ \"${SHA}\" = {} ]", sh_squote(digest)),
            &format!(
                "echo {}",
                sh_squote(&format!("exec_sha256:{name}:{digest}"))
            ),
            &diverged(
                &format!("exec_sha256_diverged:{name}:declared={digest}:live="),
                &format!("\"${SHA}\""),
            ),
        ));
    }
    out
}

/// The state-query lines: the live program and its digest, for the tripwire.
///
/// Empty when nothing is declared, so an undeclared service's observation is
/// byte-identical to what it was before PMAT-560 and no digest on the fleet
/// moves. Once declared, a program swapped after apply moves the digest, and
/// `forjar drift` sees the swap without waiting for a check.
pub fn query_lines(resource: &Resource, name: &str) -> String {
    if !resource.exec.is_declared() {
        return String::new();
    }
    format!(
        "\n{}\necho \"exec_start=${LIVE}\"\necho \"exec_sha256=${SHA}\"",
        probe(name)
    )
}

/// The apply-path tail: assert parity and fail the apply when it does not hold.
///
/// Starting and enabling a unit that runs the wrong program is not
/// convergence, and the unit file is another resource's to fix — so the apply
/// stops here, with the markers on stdout and one line on stderr, rather than
/// reporting a resource converged over code nobody reviewed.
pub fn apply_tail(resource: &Resource, name: &str) -> Vec<String> {
    let assertions = assertions(resource, name);
    if assertions.is_empty() {
        return Vec::new();
    }
    let mut out = vec![verdict::flag_init()];
    out.extend(assertions);
    out.push(verdict::exit_if_diverged(&format!(
        "FORJAR_FAIL: {name} executes something other than what was declared \
         (exec_start / exec_sha256); the exec_*_diverged line above names what it runs"
    )));
    out
}
