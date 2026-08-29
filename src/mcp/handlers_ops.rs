//! Handlers for the verbs discharged from `Bucket::Pending` (#356).
//!
//! Every one of these is a PROJECTION of a calculation that already shipped as
//! a CLI leaf. None of them computes anything the CLI does not, and none of
//! them writes: `Effects::ReadOnly` in the verb table is a promise about these
//! two functions, and `tests/falsification_verb_readonly_surface.rs` is what
//! keeps it from becoming a comment.
//!
//! There was a third — `PolicyCoverageHandler` — and it is gone rather than
//! parked here unregistered. The calculation it projected is wrong about rule
//! identity (paiml/forjar#369), so the verb was withdrawn and the leaf put back
//! in `Bucket::Pending`; a handler that no `verb_table!` row names is the dead
//! module #356 was opened to remove, not a head start on re-adding it.

use pforge_runtime::Handler;
use std::path::{Path, PathBuf};

use crate::tripwire::audit_trail;

use super::types::*;

/// MCP handler for the provenance audit trail (FJ-341).
pub struct AuditHandler;
/// MCP handler for workspace introspection (FJ-210).
pub struct WorkspaceHandler;

/// Entries returned when the caller names no limit — the same default as
/// `forjar audit -n`.
const DEFAULT_AUDIT_LIMIT: usize = 20;

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

        // The directory the selection DESIGNATES — which is not the directory
        // any verb on this surface reads. `mcp::paths::resolve_state_dir_opt`
        // above never joins `active`, so a caller that wants what `forjar plan`
        // sees under this selection has to pass this path back as `state_dir`.
        // paiml/forjar#367.
        let workspace_state_dir = match active.as_deref() {
            Some(ws) => state_base.join(ws),
            None => state_base.clone(),
        };

        Ok(WorkspaceOutput {
            active,
            workspaces: found
                .into_iter()
                .map(|w| WorkspaceEntryOutput {
                    name: w.name,
                    active: w.active,
                })
                .collect(),
            // `exists()` before `display()`: the two fields answer different
            // questions and only one of them touches the disk.
            state_base_exists: state_base.exists(),
            state_base: state_base.display().to_string(),
            workspace_state_dir: workspace_state_dir.display().to_string(),
        })
    }
}
