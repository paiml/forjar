//! Which external binaries forjar's resources are allowed to move bytes with.
//!
//! The Sovereign AI Stack ships `copia` and documents it fleet-wide as *"the
//! rsync replacement"*. Three forjar resources shell out to something else:
//! `backup_sync` (rclone) and `model` (curl) are out of copia's domain
//! entirely, and `nas_archive` (rsync) is the one remaining debt, held open
//! only by copia#46. This module is the difference between knowing that and
//! enforcing it.
//!
//! The fourth, `build`, was the debt this partition was created to make
//! visible: it pulled a cross-compiled binary back with `scp` where
//! `copia sync` works today. forjar#290 paid it, and the row is gone rather
//! than kept as a standing exception.
//!
//! # Why this exists (five whys, 2026-08-22)
//!
//! 1. `nas_archive` uses rsync, not copia — it needs
//!    `rsync -a --checksum --dry-run --itemize-changes`, a pass proving source
//!    and destination are byte-identical before the source is deleted.
//!    `copia sync` has no verify, checksum, dry-run or itemize option.
//! 2. It needs that because the resource's contract is *move 755 GB, then
//!    delete the originals*. Deleting on the strength of an exit code is the
//!    failure mode that loses data — and it already happened here: the
//!    predecessor printed `verified: 0 files differ` when rsync itself had
//!    failed.
//! 3. copia lacks it because its roadmap targeted incremental (L1), bidirectional
//!    (L2) and hub (L3) sync — transfer throughput and correctness. Archival,
//!    where the source is destroyed afterwards, was never a use case it was given.
//! 4. It was never given because the resource was built against the tool that
//!    already had the flag, rather than treating "the sovereign stack is missing
//!    a capability" as the finding. That made the resource shippable in one PR
//!    and made copia's gap invisible.
//! 5. That went unchallenged because **nothing in the build asserted it**. The
//!    policy lived in CLAUDE.md as prose, and prose is not a gate: no check
//!    counted external sync binaries, so the path of least resistance won
//!    silently and each instance made the next easier to justify.
//!
//! Root cause: a documented-but-unenforced requirement. So it is enforced here,
//! as a TOTAL partition — every external sync binary that appears in
//! `src/resources/` must be listed with a reason, and a new one fails the build.
//! That is the same shape as the CLI-leaf partition in `crate::verb::partition`,
//! and for the same reason: an exclusion list that is green by construction
//! proves nothing.

/// Why a resource is permitted to use a non-sovereign tool to move bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justification {
    /// copia cannot address this problem domain at all. Not a sovereignty gap.
    OutOfDomain,
    /// copia SHOULD cover this and cannot yet. This is DEBT and must shrink;
    /// the string cites the issue that closes it.
    Debt(&'static str),
}

/// One external binary a resource moves bytes with.
#[derive(Debug, Clone, Copy)]
pub struct SyncTool {
    /// Binary name as invoked.
    pub binary: &'static str,
    /// Resource module that invokes it.
    pub resource: &'static str,
    pub justification: Justification,
    /// One line, in full sentences. Read by whoever inherits this decision.
    pub reason: &'static str,
}

/// Every external sync binary forjar's resources invoke, exactly once.
pub fn sync_tools() -> &'static [SyncTool] {
    &[
        SyncTool {
            binary: "rclone",
            resource: "backup_sync",
            justification: Justification::OutOfDomain,
            reason: "backup_sync moves the NAS to Google Drive. copia has no cloud \
                     backends and is not trying to have any, so this is a different \
                     problem domain rather than a sovereignty gap.",
        },
        SyncTool {
            binary: "curl",
            resource: "model",
            justification: Justification::OutOfDomain,
            reason: "model downloads a weights file from an HTTPS URL. copia synchronises \
                     between filesystems and SSH hosts; it is not an HTTP client and is not \
                     trying to be one. Tracked separately as an undeclared dependency \
                     (forjar GH-224), which is a different problem from sovereignty.",
        },
        SyncTool {
            binary: "rsync",
            resource: "nas_archive",
            justification: Justification::Debt("paiml/copia#46"),
            reason: "nas_archive is local->NAS, which IS copia's domain. It stays on \
                     rsync only for the verify-before-delete pass: copia 0.2.0 cannot \
                     express a content-comparing, provably read-only diff, and \
                     migrating without one would REMOVE the property that protects \
                     755 GB. Swap it the day copia#46 lands, not before.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    /// Binaries that move bytes. A resource invoking one of these is making a
    /// sovereignty decision whether or not it knows it.
    const KNOWN_SYNC_BINARIES: &[&str] =
        &["rsync", "rclone", "scp", "sftp", "wget", "curl", "copia"];

    /// copia is the sovereign tool; using it needs no justification.
    const SOVEREIGN: &[&str] = &["copia"];

    fn resource_sources() -> Vec<(String, String)> {
        fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    // Tests describe tools they probe for; only NON-test source
                    // is evidence that the resource itself invokes one.
                    let name = p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    // Tests describe tools they probe for; only NON-test source
                    // is evidence that the resource itself invokes one.
                    if name.contains("test") {
                        continue;
                    }
                    // And skip THIS file. It is the policy, not a resource: its
                    // KNOWN_SYNC_BINARIES const names every binary in quotes, so
                    // scanning it makes the gate detect itself and report four
                    // invocations that do not exist. Caught by the gate on its
                    // first run, which is the correct way to find out.
                    if name == "sync_tools.rs" {
                        continue;
                    }
                    if let Ok(s) = fs::read_to_string(&p) {
                        out.push((p.to_string_lossy().to_string(), s));
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/resources")
                .as_path(),
            &mut out,
        );
        out
    }

    /// Strip `//` and `//!` comment bodies. A comment EXPLAINING a past rsync
    /// bug is not an invocation, and counting it would make this gate fire on
    /// documentation — training people to delete the explanation.
    fn code_only(src: &str) -> String {
        src.lines()
            .map(|l| {
                let t = l.trim_start();
                if t.starts_with("//") || t.starts_with("#") {
                    ""
                } else {
                    l
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn every_external_sync_binary_is_justified() {
        let declared: BTreeSet<&str> = sync_tools().iter().map(|t| t.binary).collect();
        let mut found: BTreeSet<String> = BTreeSet::new();

        for (path, src) in resource_sources() {
            let code = code_only(&src);
            for bin in KNOWN_SYNC_BINARIES {
                if SOVEREIGN.contains(bin) {
                    continue;
                }
                // Match an invocation, not a mention: the binary name inside a
                // string literal is how these are shelled out.
                if code.contains(&format!("\"{bin}\""))
                    || code.contains(&format!("{bin} "))
                        && (code.contains("Command::new") || code.contains("push_str"))
                {
                    let _ = &path;
                    found.insert((*bin).to_string());
                }
            }
        }

        let undeclared: Vec<&String> = found
            .iter()
            .filter(|b| !declared.contains(b.as_str()))
            .collect();
        assert!(
            undeclared.is_empty(),
            "{} external sync binary/binaries are invoked by a resource with no entry in \
             src/resources/sync_tools.rs: {:?}\n\
             The Sovereign AI Stack ships copia as the rsync replacement. Using something \
             else is a decision that must be written down with a reason — either \
             OutOfDomain (copia cannot address this) or Debt (copia should, and an issue \
             tracks it). Prose in CLAUDE.md is not a gate; this is.",
            undeclared.len(),
            undeclared
        );
    }

    #[test]
    fn the_partition_has_no_stale_entries() {
        // A declaration for a tool nothing uses any more reads as a live
        // exception and quietly widens the policy.
        let all: String = resource_sources()
            .iter()
            .map(|(_, s)| code_only(s))
            .collect::<Vec<_>>()
            .join("\n");
        for t in sync_tools() {
            assert!(
                all.contains(t.binary),
                "sync_tools declares `{}` for {}, but no resource invokes it any more — \
                 delete the entry rather than leaving a standing exception",
                t.binary,
                t.resource
            );
        }
    }

    #[test]
    fn every_entry_carries_a_real_reason() {
        for t in sync_tools() {
            assert!(
                t.reason.len() > 60,
                "{}: the reason must explain the decision to whoever inherits it",
                t.binary
            );
            if let Justification::Debt(issue) = t.justification {
                assert!(
                    issue.contains('#'),
                    "{}: Debt must cite the issue that closes it, got `{}`",
                    t.binary,
                    issue
                );
            }
        }
    }

    /// Falsification: the detector must actually SEE an invocation. If it
    /// cannot, `every_external_sync_binary_is_justified` passes over an empty
    /// set and proves nothing — the exact vacuous-green shape this gate exists
    /// to prevent elsewhere.
    #[test]
    fn the_detector_finds_the_invocations_that_are_really_there() {
        // Must be found in a REAL resource, not in this file's own const list.
        // Before sync_tools.rs was excluded from the scan, the gate detected its
        // own KNOWN_SYNC_BINARIES and reported curl/scp/sftp/wget as invoked —
        // and this assertion would have passed on that same self-reference,
        // which would have made it decoration.
        let sources = resource_sources();
        let hits: Vec<&String> = sources
            .iter()
            .filter(|(_, s)| code_only(s).contains("rsync"))
            .map(|(p, _)| p)
            .collect();
        assert!(
            hits.iter().any(|p| p.contains("nas_archive")),
            "the scanner found no rsync in nas_archive's non-test source — it is looking \
             in the wrong place, and every assertion built on it is vacuous. Hits: {hits:?}"
        );
        assert!(
            !hits.iter().any(|p| p.ends_with("sync_tools.rs")),
            "the scanner is reading its own policy file — it would detect the binaries it \
             merely NAMES as ones a resource invokes"
        );
        assert!(
            !resource_sources().is_empty(),
            "no resource sources were read at all"
        );
    }
}
