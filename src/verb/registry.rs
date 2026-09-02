//! THE table of forjar verbs. One declaration; every transport derives.
//!
//! Adding a verb is one row here. If a transport needs something the row does
//! not carry, add a field to [`VerbSpec`] — do not add a second list.

use super::spec::{Effects, VerbSpec};
use crate::mcp::handlers::*;
use crate::mcp::handlers_drift::DriftHandler;
use crate::mcp::handlers_ops::*;
use crate::mcp::types::*;
use pforge_runtime::Handler;

/// Declare the verb table. The macro exists so a row cannot be half-written:
/// name, description, effects, timeout and BOTH schemas are positional, so
/// omitting one is a compile error rather than a silently absent schema.
macro_rules! verb_table {
    ($( $name:literal, $eff:expr, $timeout:literal, $desc:literal, $input:ty, $output:ty, $handler:expr ; )*) => {
        /// Every forjar verb, in a stable order.
        pub fn verbs() -> Vec<VerbSpec> {
            vec![$(
                VerbSpec {
                    name: $name,
                    description: $desc,
                    effects: $eff,
                    timeout_ms: $timeout,
                    input_schema: || {
                        serde_json::to_value(schemars::schema_for!($input))
                            .unwrap_or(serde_json::Value::Null)
                    },
                    output_schema: || {
                        serde_json::to_value(schemars::schema_for!($output))
                            .unwrap_or(serde_json::Value::Null)
                    },
                    invoke: |params: serde_json::Value| -> Result<serde_json::Value, String> {
                        // FVS-2: params are validated BEFORE the handler runs.
                        // The validator is `$input` itself — the same type
                        // `schema_for!` derives `input_schema` from — so a
                        // params value that deserialises is exactly a params
                        // value the published schema accepts. There is no
                        // second, hand-maintained validator to drift from it.
                        let input: $input = serde_json::from_value(params)
                            .map_err(|e| format!("invalid params for `{}`: {e}", $name))?;
                        let rt = tokio::runtime::Runtime::new()
                            .map_err(|e| format!("tokio runtime: {e}"))?;
                        let out = rt
                            .block_on(async move { $handler.handle(input).await })
                            .map_err(|e| format!("{e}"))?;
                        // FVS-3: a success serialises through `$output`, the
                        // type `output_schema` is derived from.
                        serde_json::to_value(out)
                            .map_err(|e| format!("result did not serialise: {e}"))
                    },
                },
            )*]
        }
    };
}

// EVERY row is ReadOnly, and that is worth publishing rather than assuming: it
// means an MCP agent may call any forjar verb unattended without risking a
// change to a machine — any machine, the one running the verb included.
// `apply`, `destroy` and friends are deliberately NOT here — see
// `partition.rs`, where every one of them is accounted for.
//
// That claim is a constraint on the ROWS BELOW, not just on the effects column,
// and it has been broken from both sides.
//
// A verb is ReadOnly only if nothing in its `$input` type can make it run a
// subprocess: `lint` published a `policy_dir` field that fed compliance packs
// to `sh -c`, so for the life of that field the sentence above was false while
// `readOnlyHint: true` went on being published (#356). Adding a field to an
// input type is therefore an amendment to this contract, and it is stated
// identically in `spec::Effects` and in
// `core::quality_gate::GateThresholds::policy_dir`.
//
// A clean input type is not sufficient either: until 1.21.1, `plan` ran the
// CONFIG's own `ambient_inputs`, `sops`/`op` and `output_equivalence` commands,
// so calling this surface on an untrusted repository executed that repository
// with no parameter involved (forjar#372). `plan` is still ReadOnly — it
// changes no machine, and reclassifying it would have thrown away the only
// accurate signal an agent has — but it now plans over
// `core::unattended::sanitize_config`'s output and says in its result what it
// declined to run.
//
// Neither assertion is in this file, because a test that reads `verbs()` can
// only check the declaration. `falsification_read_only_verbs_do_not_write`
// drives every row through `forjar mcp` over stdio, against a booby-trapped
// fixture, and fails on any filesystem change;
// `falsification_readonly_surface_executes_nothing` drives the same rows
// against a config that tries.
//
// The guarantee is asserted, not merely written down:
// `tests/falsification_verb_readonly_surface.rs` fails the moment a row says
// `Effects::Mutating`. Ending the guarantee is a decision for a human, and
// editing that test is how a human makes it — not a side effect of adding a
// row here. A NEW row must also be answerable without executing what the
// config declares: `falsification_readonly_surface_executes_nothing.rs`
// drives every advertised verb, so a row whose handler probes is caught
// there.
//
// One row had to argue for its place. `remediate` CORRECTS a config — and still
// does not write: it returns the corrected document and the caller performs the
// write. That is not squeamishness. `src/verb/http.rs:57` prints, at runtime, on
// any non-loopback bind: "it has NO authentication. Every forjar verb is
// read-only, so this exposes configuration, not control." A mutating remediate
// would turn that printed sentence into a falsehood and an unauthenticated TCP
// port into a config-rewrite endpoint.
//
// `policy-coverage` IS NOT HERE, and its absence is deliberate rather than
// forgotten. It shipped as a row on the e4 branch, was found to answer wrongly,
// and was withdrawn: `display_id_of(None, message)` derives a rule's identity
// from its `message:`, so two rules declared without an `id:` that share a
// message collapsed to one. Measured on the built binary — two such rules, one
// violated and one satisfied — the report was `"total_rules": 2,
// "rules_triggered": 1, "untriggered_rules": []`. Two is not one plus zero: a
// rule that never ran was reported as having run, in the one report whose job
// is to say what is NOT covered. That is paiml/forjar#369.
//
// #369 IS FIXED — `policy_coverage::trigger_split` splits by rule index and
// names an idle rule with `PolicyRule::display_id_at`. The row is still in
// `Bucket::Pending` because re-adding it publishes a new tool schema on every
// transport at once and has to answer to the verb-surface suites; that is a
// decision to take on its own, and `tests/falsification_policy_coverage_
// withdrawn.rs` is where a human takes it.
verb_table! {
    "validate", Effects::ReadOnly, 30_000, "Validate a forjar.yaml configuration file", ValidateInput, ValidateOutput, ValidateHandler;
    "plan",     Effects::ReadOnly, 60_000, "Show execution plan for infrastructure changes", PlanInput, PlanOutput, PlanHandler;
    "drift",    Effects::ReadOnly, 60_000, "Detect configuration drift from desired state", DriftInput, DriftOutput, DriftHandler;
    "lint",     Effects::ReadOnly, 30_000, "Quality gate: shell safety, plaintext secrets, script complexity and compliance rules, with SARIF diagnostics", LintInput, LintOutput, LintHandler;
    "graph",    Effects::ReadOnly, 10_000, "Generate resource dependency graph (format: mermaid or dot only)", GraphInput, GraphOutput, GraphHandler;
    "show",     Effects::ReadOnly, 30_000, "Show fully resolved config with templates expanded", ShowInput, ShowOutput, ShowHandler;
    "status",   Effects::ReadOnly, 10_000, "Show current state from lock files", StatusInput, StatusOutput, StatusHandler;
    "trace",    Effects::ReadOnly, 30_000, "View trace provenance data from apply runs", TraceInput, TraceOutput, TraceHandler;
    "anomaly",  Effects::ReadOnly, 30_000, "Detect anomalous resource behavior using ML-inspired analysis", AnomalyInput, AnomalyOutput, AnomalyHandler;
    "remediate", Effects::ReadOnly, 30_000, "Compute policy-derived corrections to a forjar.yaml and return the corrected document (never writes)", RemediateInput, RemediateOutput, RemediateHandler;
    "audit",    Effects::ReadOnly, 30_000, "Read the append-only provenance trail recorded by apply runs", AuditInput, AuditOutput, AuditHandler;
    "workspace", Effects::ReadOnly, 10_000, "Report which workspace the forjar CLI has selected and every workspace under the state dir; the selection does not change where the other verbs read state", WorkspaceInput, WorkspaceOutput, WorkspaceHandler;
}

/// Look up a verb by its transport-neutral name.
pub fn find(name: &str) -> Option<VerbSpec> {
    verbs().into_iter().find(|v| v.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // A registry that silently empties makes every downstream parity
    // assertion vacuous: "the CLI and MCP expose the same names" is trivially
    // true of two empty sets. Assert a floor, and assert it against a literal
    // rather than against `verbs().len()`.
    #[test]
    fn registry_is_not_empty() {
        assert!(
            verbs().len() >= 12,
            "verb registry collapsed to {} entries — every parity test downstream \
             becomes vacuous when this is empty",
            verbs().len()
        );
    }

    #[test]
    fn names_are_unique() {
        let v = verbs();
        let uniq: HashSet<_> = v.iter().map(|s| s.name).collect();
        assert_eq!(uniq.len(), v.len(), "duplicate verb name in the registry");
    }

    #[test]
    fn mcp_names_are_derived_not_typed() {
        for v in verbs() {
            assert_eq!(v.mcp_name(), format!("forjar_{}", v.name.replace('-', "_")));
        }
    }

    #[test]
    fn every_verb_has_real_schemas() {
        for v in verbs() {
            let i = (v.input_schema)();
            let o = (v.output_schema)();
            assert!(
                i.is_object() && !i.as_object().unwrap().is_empty(),
                "{}: input schema is empty — a schema that is Null validates \
                 nothing, so FVS-2 would pass while checking no params",
                v.name
            );
            assert!(
                o.is_object() && !o.as_object().unwrap().is_empty(),
                "{}: output schema is empty",
                v.name
            );
        }
    }

    #[test]
    fn read_only_hint_is_derived_from_effects() {
        assert!(Effects::ReadOnly.read_only());
        assert!(!Effects::Mutating.read_only());
    }
}
