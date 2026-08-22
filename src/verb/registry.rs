//! THE table of forjar verbs. One declaration; every transport derives.
//!
//! Adding a verb is one row here. If a transport needs something the row does
//! not carry, add a field to [`VerbSpec`] — do not add a second list.

use super::spec::{Effects, VerbSpec};
use crate::mcp::handlers::*;
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

// All nine are ReadOnly, and that is worth publishing rather than assuming:
// it means an MCP agent may call any forjar verb unattended without risking a
// change to a machine. `apply`, `destroy` and friends are deliberately NOT
// here — see `partition.rs`, where every one of them is accounted for.
verb_table! {
    "validate", Effects::ReadOnly, 30_000, "Validate a forjar.yaml configuration file", ValidateInput, ValidateOutput, ValidateHandler;
    "plan",     Effects::ReadOnly, 60_000, "Show execution plan for infrastructure changes", PlanInput, PlanOutput, PlanHandler;
    "drift",    Effects::ReadOnly, 60_000, "Detect configuration drift from desired state", DriftInput, DriftOutput, DriftHandler;
    "lint",     Effects::ReadOnly, 30_000, "Lint forjar config for best practices and shell safety", LintInput, LintOutput, LintHandler;
    "graph",    Effects::ReadOnly, 10_000, "Generate resource dependency graph (format: mermaid or dot only)", GraphInput, GraphOutput, GraphHandler;
    "show",     Effects::ReadOnly, 30_000, "Show fully resolved config with templates expanded", ShowInput, ShowOutput, ShowHandler;
    "status",   Effects::ReadOnly, 10_000, "Show current state from lock files", StatusInput, StatusOutput, StatusHandler;
    "trace",    Effects::ReadOnly, 30_000, "View trace provenance data from apply runs", TraceInput, TraceOutput, TraceHandler;
    "anomaly",  Effects::ReadOnly, 30_000, "Detect anomalous resource behavior using ML-inspired analysis", AnomalyInput, AnomalyOutput, AnomalyHandler;
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
            verbs().len() >= 9,
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
            assert_eq!(v.mcp_name(), format!("forjar_{}", v.name));
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
