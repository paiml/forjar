//! Handlers for the verbs discharged from `Bucket::Pending` (#356).
//!
//! Every one of these is a PROJECTION of a calculation that already shipped as
//! a CLI leaf. None of them computes anything the CLI does not, and none of
//! them writes: `Effects::ReadOnly` in the verb table is a promise about these
//! three functions, and `tests/falsification_verb_readonly_surface.rs` is what
//! keeps it from becoming a comment.

use pforge_runtime::Handler;
use std::path::{Path, PathBuf};

use crate::core::{parser, policy_coverage};
use crate::tripwire::audit_trail;

use super::types::*;

/// MCP handler for policy rule coverage (FJ-3208).
pub struct PolicyCoverageHandler;
/// MCP handler for the provenance audit trail (FJ-341).
pub struct AuditHandler;
/// MCP handler for workspace introspection (FJ-210).
pub struct WorkspaceHandler;

/// Entries returned when the caller names no limit — the same default as
/// `forjar audit -n`.
const DEFAULT_AUDIT_LIMIT: usize = 20;

#[async_trait::async_trait]
impl Handler for PolicyCoverageHandler {
    type Input = PolicyCoverageInput;
    type Output = PolicyCoverageOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);
        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;
        let cov = policy_coverage::compute_coverage(&config);

        // Sorted, because a set has no order and an MCP client diffing two
        // responses would otherwise see a change that is not one.
        let mut frameworks: Vec<String> = cov.frameworks.iter().cloned().collect();
        frameworks.sort();

        Ok(PolicyCoverageOutput {
            total_resources: cov.total_resources,
            covered_resources: cov.covered_resources,
            coverage_percent: cov.coverage_percent(),
            fully_covered: cov.fully_covered(),
            uncovered: cov.uncovered.clone(),
            by_type: cov.by_type.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            frameworks,
        })
    }
}

#[async_trait::async_trait]
impl Handler for AuditHandler {
    type Input = AuditInput;
    type Output = AuditOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let state_dir =
            super::paths::resolve_state_dir_opt(input.path.as_deref(), input.state_dir.as_deref());
        let limit = input.limit.unwrap_or(DEFAULT_AUDIT_LIMIT);

        // An unreadable state dir is an ERROR, not an empty trail. GH-208 is
        // the reason: "I could not find your state" reported as "nothing
        // happened" is the failure mode that let `forjar_drift` certify a
        // tampered machine as clean, and an audit tool that answers "no events"
        // when it never opened the log is the same defect wearing a different
        // name.
        let events = audit_trail::collect_events(&state_dir, input.machine.as_deref(), limit)
            .map_err(pforge_runtime::Error::Handler)?;

        let events: Vec<AuditEventOutput> = events
            .into_iter()
            .map(|(machine, ev)| {
                let event = serde_json::to_value(&ev.event).unwrap_or(serde_json::Value::Null);
                AuditEventOutput {
                    machine,
                    timestamp: ev.ts,
                    event,
                }
            })
            .collect();

        Ok(AuditOutput {
            event_count: events.len(),
            events,
        })
    }
}

#[async_trait::async_trait]
impl Handler for WorkspaceHandler {
    type Input = WorkspaceInput;
    type Output = WorkspaceOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        // `.forjar/workspace` lives beside the config, NOT in the server's cwd.
        // The CLI hard-codes `Path::new(".")` and is right to: its cwd IS the
        // project. An MCP server's cwd is chosen by the client (GH-208).
        let root: PathBuf = input
            .path
            .as_deref()
            .and_then(|p| Path::new(p).parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        let state_base =
            super::paths::resolve_state_dir_opt(input.path.as_deref(), input.state_dir.as_deref());

        let found = crate::cli::workspace::list_workspaces_in(&root, &state_base)
            .map_err(pforge_runtime::Error::Handler)?;
        let active = crate::cli::workspace::current_workspace_in(&root);

        Ok(WorkspaceOutput {
            active,
            workspaces: found
                .into_iter()
                .map(|w| WorkspaceEntryOutput {
                    name: w.name,
                    active: w.active,
                })
                .collect(),
            state_base: state_base.display().to_string(),
        })
    }
}
