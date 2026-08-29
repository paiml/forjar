//! The `forjar_remediate` handler (paiml/forjar#356).
//!
//! Split out of `handlers.rs`, which is already near the repo's file ceiling,
//! following the `handlers_state.rs` precedent.
//!
//! It reads the config, computes the corrections its policies determine, and
//! returns the corrected document. **It performs no write.** That is what keeps
//! the verb `Effects::ReadOnly`, and the read-only property is load-bearing:
//! `verb serve` tells the operator, at runtime, that every forjar verb is
//! read-only and therefore an unauthenticated bind exposes configuration rather
//! than control.

use super::handlers::RemediateHandler;
use super::types::*;
use crate::core::parser;
use crate::core::remediate::{self, Report};
use pforge_runtime::Handler;
use std::path::PathBuf;

#[async_trait::async_trait]
impl Handler for RemediateHandler {
    type Input = RemediateInput;
    type Output = RemediateOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);
        // The SOURCE text, not the resolved config: the edit lands in the file
        // the operator wrote, so includes and recipe expansions stay where they
        // are instead of being inlined by a re-serialisation.
        let source = std::fs::read_to_string(&path).map_err(|e| {
            pforge_runtime::Error::Handler(format!("cannot read {}: {e}", path.display()))
        })?;
        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;
        let report = remediate::remediate(&source, &config, input.policy_ids.as_deref())
            .map_err(pforge_runtime::Error::Handler)?;
        Ok(project(report))
    }
}

/// Map the core report onto the published output type.
fn project(report: Report) -> RemediateOutput {
    RemediateOutput {
        remediations_applied: report
            .applied
            .into_iter()
            .map(|f| RemediationOutput {
                policy_id: f.policy_id,
                resource_id: f.resource_id,
                field: f.field,
                from: f.from,
                to: f.to,
                line: f.line,
            })
            .collect(),
        updated_yaml_content: report.updated_yaml,
        remaining_violations: report
            .remaining
            .into_iter()
            .map(|v| ViolationOutput {
                policy_id: v.policy_id,
                resource_id: v.resource_id,
                message: v.message,
                severity: v.severity,
                rule_type: v.rule_type,
                remediation_hint: v.remediation_hint,
                reason: v.reason,
            })
            .collect(),
        changed: report.changed,
        config_hash_before: report.hash_before,
        config_hash_after: report.hash_after,
        scope_note: report.scope_note,
    }
}
