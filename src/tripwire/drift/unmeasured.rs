//! forjar#549: a query the target never answered is UNMEASURED, not drift.
//!
//! # The defect
//!
//! Every detector asked the target through `transport::exec_script_timeout` and
//! read both failure arms as verdicts about the target. Over SSH an unreachable
//! host does not even produce `Err`: `ssh` exits 255 and prints `connect to host
//! … Connection timed out`, and the file detector rendered that as
//! `actual: MISSING` — the word a REACHED host earns for a file that is really
//! gone. Measured on 1.29.0 against 203.0.113.9 (TEST-NET-3):
//!
//! ```text
//! drift_count 1   actual_hash MISSING
//! detail "… not accessible: ssh: connect to host 203.0.113.9 port 22: Connection timed out"
//! ```
//!
//! A timeout (`Err`) became `ERROR`, and a task guard whose check never ran was
//! reported as drift too. A fleet tripwire that cannot tell "the file changed"
//! from "the switch is down" pages for the wrong thing, and its reader learns to
//! ignore it.
//!
//! # One reader
//!
//! [`read`] is the only place a detector learns whether the target answered, so
//! the four detectors cannot disagree about it. `Err` — the spawn failed, the
//! 60s bound fired, a container or pepita transport failed — is unmeasured. So
//! is exit 255 over SSH: `ssh(1)` "exits with the exit status of the remote
//! command or with 255 if an error occurred" (connect, DNS, host key,
//! authentication). A remote script that itself exits 255 is read as unmeasured
//! as well. That errs toward "not known", which `--tripwire` still fails on
//! (exit 4), and never toward a clean pass.

use super::census::DriftCensus;
use super::{DriftFinding, DRIFT_QUERY_TIMEOUT_SECS};
use crate::core::types::{Machine, ResourceType};
use crate::transport::ExecOutput;

/// The `actual_hash` of a finding whose query the target never answered.
pub const UNMEASURED: &str = "UNMEASURED";

/// `ssh(1)` reserves this exit status for its own failures.
const SSH_OWN_FAILURE: i32 = 255;

/// What asking the target produced.
pub(super) enum Reading {
    /// The target ran the query: its exit code and output are evidence about it.
    Answered(ExecOutput),
    /// Nothing about the target was learned. The string says why.
    Unmeasured(String),
}

/// Run one drift query on the target, under the drift bound, and classify it.
pub(super) fn read(machine: &Machine, script: &str) -> Reading {
    let result =
        crate::transport::exec_script_timeout(machine, script, Some(DRIFT_QUERY_TIMEOUT_SECS));
    classify(machine, result)
}

/// The rule without the I/O, so every arm can be tested.
pub(super) fn classify(machine: &Machine, result: Result<ExecOutput, String>) -> Reading {
    match result {
        Err(e) => Reading::Unmeasured(format!("transport error: {e}")),
        Ok(out)
            if out.exit_code == SSH_OWN_FAILURE && crate::transport::is_ssh_transport(machine) =>
        {
            Reading::Unmeasured(format!(
                "ssh to {} failed before the query ran (exit 255): {}",
                machine.addr,
                out.stderr.trim()
            ))
        }
        Ok(out) => Reading::Answered(out),
    }
}

impl DriftFinding {
    /// A resource whose query the target never answered: neither clean nor drifted.
    pub fn unmeasured(
        resource_id: &str,
        resource_type: ResourceType,
        expected_hash: &str,
        detail: String,
    ) -> Self {
        Self {
            resource_id: resource_id.to_string(),
            resource_type,
            expected_hash: expected_hash.to_string(),
            actual_hash: UNMEASURED.to_string(),
            detail,
        }
    }

    /// Does this finding record an unanswered query rather than a difference?
    pub fn is_unmeasured(&self) -> bool {
        self.actual_hash == UNMEASURED
    }
}

/// Tell the census which resources went unmeasured.
///
/// Runs once, after every detector. Detectors record `inspected` as they go and
/// a file resource is inspected by two of them, so marking it any earlier would
/// be overwritten by the second.
pub(super) fn census_unmeasured(findings: &[DriftFinding], census: &mut DriftCensus) {
    for f in findings.iter().filter(|f| f.is_unmeasured()) {
        census.unmeasured(&f.resource_id, &f.resource_type);
    }
}
