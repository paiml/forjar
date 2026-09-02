//! The total partition of forjar's CLI leaves.
//!
//! FVS does NOT promise to unify 193 leaves. A migration promise that large is
//! unfalsifiable, and the sibling project that wrote this spec first has not
//! kept it. What it promises instead is that **every leaf is in exactly one
//! bucket, with a written reason**, and that the partition is TOTAL: a new CLI
//! leaf that names no bucket fails the build.
//!
//! That last property is the whole point. A judge panel reviewing this design
//! named its weakest link precisely: the load-bearing half is a NEGATIVE claim
//! ("the other 184 are deliberately out of scope"), and an exclusion list that
//! is green by construction proves nothing. So the test below does not read
//! this table and check it against itself — it walks the LIVE clap tree and
//! asserts set equality in both directions. Adding a subcommand without
//! bucketing it is a red test; deleting one without removing its row is too.

use std::collections::BTreeSet;

/// Which surface a CLI leaf belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bucket {
    /// On every declared transport, parity- and invariance-gated.
    Unified,
    /// Deliberately CLI-shaped. Carries the reason it is not a verb.
    CliOnly(&'static str),
    /// Belongs on the unified surface, is not there yet. Carries an issue ref.
    ///
    /// This is the debt ledger. It normally only shrinks — but a row may come
    /// BACK, and `policy-coverage` is the one that has. It shipped as a verb on
    /// this branch and was withdrawn when the unified calculation was measured
    /// answering wrongly (paiml/forjar#369). Honest debt is a smaller failure
    /// than a published tool that is confidently wrong, so a return is allowed
    /// on exactly one condition: the reason names the defect, not the intent.
    Pending(&'static str),
}

/// One CLI leaf and its bucket. `path` is the full argv path, so a nested
/// leaf like `rules serve` is `&["rules", "serve"]` — the reason a flat list
/// of names could not express this partition correctly.
#[derive(Debug, Clone)]
pub struct Leaf {
    pub path: &'static [&'static str],
    pub bucket: Bucket,
}

/// Every CLI leaf, exactly once.
pub fn partition() -> &'static [Leaf] {
    PARTITION
}

/// Leaves on the unified surface, by name.
///
/// A NESTED leaf contributes its PARENT (`workspace list` -> `workspace`),
/// because a verb unifies the capability, not the argv spelling. A parent may
/// therefore be partly unified: `workspace list` and `workspace current` read,
/// so they are verbs; `workspace new`, `select` and `delete` write, so they stay
/// in the debt ledger until someone decides — deliberately, and not by adding a
/// row — that this surface may mutate a machine.
pub fn unified_names() -> BTreeSet<&'static str> {
    PARTITION
        .iter()
        .filter(|l| l.bucket == Bucket::Unified)
        .map(|l| l.path[0])
        .collect()
}

#[rustfmt::skip]
static PARTITION: &[Leaf] = &[
    Leaf { path: &["agent"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["agent-registry"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["agent-sbom"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["anomaly"], bucket: Bucket::Unified },
    Leaf { path: &["apply"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["archive", "inspect"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["archive", "pack"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["archive", "unpack"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["archive", "verify"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["audit"], bucket: Bucket::Unified },
    Leaf { path: &["bench"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["bootstrap"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["build"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["bundle"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["cache", "list"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["cache", "pull"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["cache", "push"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["cache", "verify"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["canary"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["catalog-list"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["cbom"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["check"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["checkpoint"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["codegen"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["compare"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["completion"], bucket: Bucket::CliOnly("emits a shell completion script — a terminal affordance with no transport-neutral meaning") },
    Leaf { path: &["complexity"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["compliance"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["config-merge"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["contracts"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["convert"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["cost-estimate"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["cross-deps"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["data-freshness"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["data-validate"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["dataset-lineage"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["destroy"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["diff"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["dist"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["doctor"], bucket: Bucket::CliOnly("prints host diagnostics for a human reading a terminal; its value IS the rendering") },
    Leaf { path: &["dogfood"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["drift"], bucket: Bucket::Unified },
    Leaf { path: &["drift-predict"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["env"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["env-diff"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["environments", "diff"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["environments", "history"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["environments", "list"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["environments", "rollback"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["explain"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["export"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["extract"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["fault-inject"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["fmt"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["generation", "diff"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["generation", "gc"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["generation", "list"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["graph"], bucket: Bucket::Unified },
    Leaf { path: &["history"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["image"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["impact"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["import"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["import-brownfield"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["import-makefile"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["init"], bucket: Bucket::CliOnly("scaffolds a project in the working directory, interactively — a workstation affordance") },
    Leaf { path: &["invariants"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["inventory"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["iso-export"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lineage"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lint"], bucket: Bucket::Unified },
    Leaf { path: &["lock"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-archive"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-audit"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-audit-trail"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-backup"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-compact"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-compact-all"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-compress"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-defrag"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-diff"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-export"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-gc"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-history"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-info"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-integrity"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-merge"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-migrate"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-normalize"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-prune"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-rebase"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-rehash"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-repair"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-restore"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-rotate-keys"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-sign"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-snapshot"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-stats"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-tag"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-validate"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-verify"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-verify-chain"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-verify-hmac"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-verify-schema"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lock-verify-sig"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["logs"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["lsp"], bucket: Bucket::CliOnly("speaks the Language Server Protocol on stdio — already a protocol surface, not a forjar verb") },
    Leaf { path: &["make"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["mcp"], bucket: Bucket::CliOnly("starts the MCP transport itself; a verb that launches a transport cannot be one of its own verbs") },
    Leaf { path: &["migrate"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["model-card"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["model-eval"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["multi-apply"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["oci-pack"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["output"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["parallel-apply"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["pin"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["plan"], bucket: Bucket::Unified },
    Leaf { path: &["plan-compact"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["plugin", "build"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["plugin", "init"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["plugin", "install"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["plugin", "list"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["plugin", "remove"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["plugin", "run"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["plugin", "verify"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["policy"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["policy-coverage"], bucket: Bucket::Pending("paiml/forjar#369 — WITHDRAWN after shipping on this branch, not never-started. Rule identity was derived from `message:` when a rule declares no `id:`, so two such rules sharing a message collapsed into one: measured `total_rules: 2, rules_triggered: 1, untriggered_rules: []`, which reported a rule that never ran as having run. THAT DEFECT IS FIXED — `trigger_split` splits by rule index and names an idle rule with `display_id_at`, and `tests/falsification_policy_rule_identity.rs` measures it. What remains is the re-ship itself: two type declarations, one handler, one `register_all` line and one `verb_table!` row, which publishes a new tool schema on every transport and answers to the verb-surface suites. A decision, not a leftover") },
    Leaf { path: &["policy-install"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["preservation"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["privilege-analysis"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["promote"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["prove"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["provenance"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["query"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["registry-list"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["remediate"], bucket: Bucket::Unified },
    Leaf { path: &["repro-proof"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["reseal"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["retry-failed"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["rollback"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["rolling"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["rules", "coverage"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["rules", "validate"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["run"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["saga"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["sbom"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["schema"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["score"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["secrets", "decrypt"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["secrets", "encrypt"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["secrets", "keygen"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["secrets", "rekey"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["secrets", "rotate"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["secrets", "view"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["security-scan"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["show"], bucket: Bucket::Unified },
    Leaf { path: &["sign"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["snapshot", "delete"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["snapshot", "list"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["snapshot", "restore"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["snapshot", "save"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["sovereignty"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["stack-diff"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["stack-graph"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["state-backend"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["state-decrypt"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["state-encrypt"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["state-list"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["state-mv"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["state-query"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["state-reconstruct"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["state-rekey"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["state-rm"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["status"], bucket: Bucket::Unified },
    Leaf { path: &["store", "diff"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["store", "gc"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["store", "list"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["store", "sync"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["store", "verify"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["store-import"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["suggest"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["template"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["test"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["trace"], bucket: Bucket::Unified },
    Leaf { path: &["trigger"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["undo"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["undo-destroy"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["rules", "serve"], bucket: Bucket::CliOnly("an HMAC-authenticated INBOUND webhook receiver (#205). A receiver ACCEPTS events; it does not expose forjar's capability set. Declaring it a transport would assert parity between `forjar plan` and an event endpoint, which is not a meaningful equality") },
    Leaf { path: &["verb", "call"], bucket: Bucket::CliOnly("the unified surface's own entry point; a verb that invokes verbs cannot be one of them without recursing") },
    Leaf { path: &["verb", "list"], bucket: Bucket::CliOnly("the unified surface's own entry point; a verb that invokes verbs cannot be one of them without recursing") },
    Leaf { path: &["verb", "serve"], bucket: Bucket::CliOnly("starts the HTTP transport; a verb that launches a transport cannot be one of the verbs that transport serves") },
    Leaf { path: &["verb", "schema"], bucket: Bucket::CliOnly("the unified surface's own entry point; a verb that invokes verbs cannot be one of them without recursing") },
    Leaf { path: &["validate"], bucket: Bucket::Unified },
    Leaf { path: &["verify"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["watch"], bucket: Bucket::CliOnly("long-running terminal UI that redraws; it has no single response to return") },
    Leaf { path: &["workspace", "current"], bucket: Bucket::Unified },
    Leaf { path: &["workspace", "delete"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["workspace", "list"], bucket: Bucket::Unified },
    Leaf { path: &["workspace", "new"], bucket: Bucket::Pending("paiml/forjar#288") },
    Leaf { path: &["workspace", "select"], bucket: Bucket::Pending("paiml/forjar#288") },
];

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Subcommand;

    /// Walk the LIVE clap tree the binary exposes, not a list someone typed.
    ///
    /// On a 16 MiB stack: constructing 193 `clap::Command`s recursively
    /// overflows a test thread's default 2 MiB. `main` has 8 MiB so the shipped
    /// binary is unaffected, but the test harness is not the binary — and a
    /// SIGABRT here would read as "the partition is broken" rather than "the
    /// fixture ran out of stack".
    fn live_leaves() -> BTreeSet<Vec<String>> {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(live_leaves_inner)
            .expect("spawn walker")
            .join()
            .expect("clap tree walk panicked")
    }

    fn live_leaves_inner() -> BTreeSet<Vec<String>> {
        fn walk(cmd: &clap::Command, prefix: &mut Vec<String>, out: &mut BTreeSet<Vec<String>>) {
            let subs: Vec<_> = cmd
                .get_subcommands()
                .filter(|c| c.get_name() != "help")
                .collect();
            if subs.is_empty() {
                if !prefix.is_empty() {
                    out.insert(prefix.clone());
                }
                return;
            }
            for s in subs {
                prefix.push(s.get_name().to_string());
                walk(s, prefix, out);
                prefix.pop();
            }
        }
        let root = crate::cli::Commands::augment_subcommands(clap::Command::new("forjar"));
        let mut out = BTreeSet::new();
        walk(&root, &mut Vec::new(), &mut out);
        out
    }

    fn declared_leaves() -> BTreeSet<Vec<String>> {
        PARTITION
            .iter()
            .map(|l| l.path.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    #[test]
    fn the_partition_is_total() {
        let live = live_leaves();
        let declared = declared_leaves();

        let unclassified: Vec<_> = live.difference(&declared).cloned().collect();
        assert!(
            unclassified.is_empty(),
            "{} CLI leaf/leaves exist with no bucket in src/verb/partition.rs.\n\
             Every leaf must be Unified, CliOnly(reason) or Pending(issue):\n  {}\n\
             (live={} declared={})",
            unclassified.len(),
            unclassified
                .iter()
                .map(|p| p.join(" "))
                .collect::<Vec<_>>()
                .join("\n  "),
            live.len(),
            declared.len()
        );
    }

    #[test]
    fn the_partition_has_no_stale_rows() {
        let live = live_leaves();
        let declared = declared_leaves();
        let stale: Vec<_> = declared.difference(&live).cloned().collect();
        assert!(
            stale.is_empty(),
            "{} row(s) in the partition name a leaf the CLI no longer has — a stale \n\
             row makes the totality test pass while covering something that does not \n\
             exist:\n  {}",
            stale.len(),
            stale
                .iter()
                .map(|p| p.join(" "))
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }

    #[test]
    fn every_leaf_is_in_exactly_one_bucket() {
        let mut seen = BTreeSet::new();
        for l in PARTITION {
            let key: Vec<String> = l.path.iter().map(|s| s.to_string()).collect();
            assert!(
                seen.insert(key.clone()),
                "leaf listed twice: {}",
                key.join(" ")
            );
        }
    }

    #[test]
    fn clionly_and_pending_carry_a_reason() {
        for l in PARTITION {
            match &l.bucket {
                Bucket::CliOnly(r) => assert!(
                    r.len() > 20,
                    "{}: CliOnly needs a real reason, not `{}`",
                    l.path.join(" "),
                    r
                ),
                Bucket::Pending(r) => assert!(
                    r.contains('#'),
                    "{}: Pending must cite an issue, got `{}`",
                    l.path.join(" "),
                    r
                ),
                Bucket::Unified => {}
            }
        }
    }

    /// The unified bucket must match the verb registry exactly. If these drift,
    /// the partition is describing a surface that does not exist.
    #[test]
    fn unified_bucket_matches_the_verb_registry() {
        let from_partition = unified_names();
        let from_registry: BTreeSet<&str> = crate::verb::registry::verbs()
            .iter()
            .map(|v| v.name)
            .collect();
        assert_eq!(
            from_partition, from_registry,
            "the Unified bucket and the verb registry disagree"
        );
    }

    /// Falsification: the fixture must actually contain a multi-segment leaf,
    /// or `the_partition_is_total` is only ever exercised on flat names and
    /// would not catch a nested subcommand being added unbucketed.
    #[test]
    fn the_partition_covers_nested_leaves() {
        let nested = PARTITION.iter().filter(|l| l.path.len() > 1).count();
        assert!(
            nested >= 40,
            "only {} nested leaves declared; forjar has 43 across 10 parents, so \
             the totality test is not exercising nesting",
            nested
        );
    }
}
