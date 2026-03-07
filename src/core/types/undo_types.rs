//! FJ-2003: Undo plan types — stack undo, resume, and progress tracking.
//!
//! Types for `forjar undo`: generation diff, execution plan,
//! per-resource progress, and multi-machine coordination.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// FJ-2003: Undo execution plan.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{UndoPlan, UndoResourceAction, UndoAction};
///
/// let plan = UndoPlan {
///     generation_from: 12,
///     generation_to: 10,
///     machines: vec!["intel".into()],
///     actions: vec![UndoResourceAction {
///         resource_id: "cargo-tools".into(),
///         machine: "intel".into(),
///         action: UndoAction::Destroy,
///         reversible: true,
///     }],
///     dry_run: false,
/// };
/// assert_eq!(plan.action_count(), 1);
/// assert_eq!(plan.destroy_count(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoPlan {
    /// Source generation (current).
    pub generation_from: u32,
    /// Target generation (rollback to).
    pub generation_to: u32,
    /// Affected machines.
    pub machines: Vec<String>,
    /// Ordered list of resource actions.
    pub actions: Vec<UndoResourceAction>,
    /// Whether this is a dry-run (no changes).
    pub dry_run: bool,
}

impl UndoPlan {
    /// Total number of resource actions.
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    /// Number of destroy actions.
    pub fn destroy_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|a| matches!(a.action, UndoAction::Destroy))
            .count()
    }

    /// Number of create actions.
    pub fn create_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|a| matches!(a.action, UndoAction::Create))
            .count()
    }

    /// Number of update actions.
    pub fn update_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|a| matches!(a.action, UndoAction::Update))
            .count()
    }

    /// Whether any action is irreversible.
    pub fn has_irreversible(&self) -> bool {
        self.actions.iter().any(|a| !a.reversible)
    }

    /// Format a human-readable plan summary.
    pub fn format_summary(&self) -> String {
        let mut out = format!(
            "Undo: generation {} → {}\n",
            self.generation_from, self.generation_to
        );
        out.push_str(&format!("Machines: {}\n", self.machines.join(", ")));
        out.push_str(&format!(
            "Actions: {} destroy, {} create, {} update\n",
            self.destroy_count(),
            self.create_count(),
            self.update_count(),
        ));
        if self.has_irreversible() {
            out.push_str("WARNING: Plan contains irreversible actions\n");
        }
        if self.dry_run {
            out.push_str("(dry-run — no changes will be made)\n");
        }
        for a in &self.actions {
            let tag = match a.action {
                UndoAction::Destroy => "DESTROY",
                UndoAction::Create => "CREATE",
                UndoAction::Update => "UPDATE",
            };
            let rev = if a.reversible { "" } else { " [IRREVERSIBLE]" };
            out.push_str(&format!("  [{tag}] {}/{}{rev}\n", a.machine, a.resource_id));
        }
        out
    }
}

/// A single resource action in an undo plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoResourceAction {
    /// Resource identifier.
    pub resource_id: String,
    /// Machine name.
    pub machine: String,
    /// Action to perform.
    pub action: UndoAction,
    /// Whether this action is reversible.
    pub reversible: bool,
}

/// Undo action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UndoAction {
    /// Resource was added since target gen — destroy it.
    Destroy,
    /// Resource was removed since target gen — recreate it.
    Create,
    /// Resource was modified since target gen — revert it.
    Update,
}

/// FJ-2003: Undo progress tracking for resume support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoProgress {
    /// Source generation.
    pub generation_from: u32,
    /// Target generation.
    pub generation_to: u32,
    /// ISO 8601 start timestamp.
    pub started_at: String,
    /// Overall status.
    pub status: UndoStatus,
    /// Per-resource progress.
    pub resources: HashMap<String, ResourceProgress>,
}

impl UndoProgress {
    /// Count of completed resources.
    pub fn completed_count(&self) -> usize {
        self.resources
            .values()
            .filter(|r| matches!(r.status, ResourceProgressStatus::Completed))
            .count()
    }

    /// Count of pending resources.
    pub fn pending_count(&self) -> usize {
        self.resources
            .values()
            .filter(|r| matches!(r.status, ResourceProgressStatus::Pending))
            .count()
    }

    /// Count of failed resources.
    pub fn failed_count(&self) -> usize {
        self.resources
            .values()
            .filter(|r| matches!(r.status, ResourceProgressStatus::Failed { .. }))
            .count()
    }

    /// Whether the undo is fully completed.
    pub fn is_complete(&self) -> bool {
        matches!(self.status, UndoStatus::Completed)
    }

    /// Whether resume is needed.
    pub fn needs_resume(&self) -> bool {
        matches!(self.status, UndoStatus::Partial)
    }
}

/// Overall undo operation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UndoStatus {
    /// Not yet started.
    Pending,
    /// Currently executing.
    InProgress,
    /// Partially completed (some resources failed).
    Partial,
    /// All resources completed successfully.
    Completed,
}

/// Per-resource undo progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceProgress {
    /// Resource status.
    pub status: ResourceProgressStatus,
    /// ISO 8601 timestamp of last status change.
    #[serde(default)]
    pub at: Option<String>,
}

/// Per-resource undo status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ResourceProgressStatus {
    /// Not yet processed.
    Pending,
    /// Successfully completed.
    Completed,
    /// Failed with error.
    Failed {
        /// Error message.
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_plan() -> UndoPlan {
        UndoPlan {
            generation_from: 12,
            generation_to: 10,
            machines: vec!["intel".into(), "jetson".into()],
            actions: vec![
                UndoResourceAction {
                    resource_id: "new-pkg".into(),
                    machine: "intel".into(),
                    action: UndoAction::Destroy,
                    reversible: true,
                },
                UndoResourceAction {
                    resource_id: "old-config".into(),
                    machine: "intel".into(),
                    action: UndoAction::Create,
                    reversible: true,
                },
                UndoResourceAction {
                    resource_id: "bash-aliases".into(),
                    machine: "intel".into(),
                    action: UndoAction::Update,
                    reversible: true,
                },
            ],
            dry_run: false,
        }
    }

    #[test]
    fn undo_plan_counts() {
        let plan = sample_plan();
        assert_eq!(plan.action_count(), 3);
        assert_eq!(plan.destroy_count(), 1);
        assert_eq!(plan.create_count(), 1);
        assert_eq!(plan.update_count(), 1);
    }

    #[test]
    fn undo_plan_reversible() {
        let plan = sample_plan();
        assert!(!plan.has_irreversible());
    }

    #[test]
    fn undo_plan_irreversible() {
        let mut plan = sample_plan();
        plan.actions[0].reversible = false;
        assert!(plan.has_irreversible());
    }

    #[test]
    fn undo_plan_format_summary() {
        let plan = sample_plan();
        let summary = plan.format_summary();
        assert!(summary.contains("generation 12 → 10"));
        assert!(summary.contains("intel, jetson"));
        assert!(summary.contains("1 destroy"));
        assert!(summary.contains("1 create"));
        assert!(summary.contains("1 update"));
        assert!(summary.contains("[DESTROY]"));
        assert!(summary.contains("[CREATE]"));
        assert!(summary.contains("[UPDATE]"));
    }

    #[test]
    fn undo_plan_dry_run() {
        let mut plan = sample_plan();
        plan.dry_run = true;
        let summary = plan.format_summary();
        assert!(summary.contains("dry-run"));
    }

    #[test]
    fn undo_plan_irreversible_warning() {
        let mut plan = sample_plan();
        plan.actions[0].reversible = false;
        let summary = plan.format_summary();
        assert!(summary.contains("IRREVERSIBLE"));
    }

    #[test]
    fn undo_progress_counts() {
        let mut resources = HashMap::new();
        resources.insert(
            "a".into(),
            ResourceProgress {
                status: ResourceProgressStatus::Completed,
                at: Some("2026-03-05T14:30:01Z".into()),
            },
        );
        resources.insert(
            "b".into(),
            ResourceProgress {
                status: ResourceProgressStatus::Failed {
                    error: "timeout".into(),
                },
                at: Some("2026-03-05T14:30:05Z".into()),
            },
        );
        resources.insert(
            "c".into(),
            ResourceProgress {
                status: ResourceProgressStatus::Pending,
                at: None,
            },
        );
        let progress = UndoProgress {
            generation_from: 12,
            generation_to: 10,
            started_at: "2026-03-05T14:30:00Z".into(),
            status: UndoStatus::Partial,
            resources,
        };
        assert_eq!(progress.completed_count(), 1);
        assert_eq!(progress.failed_count(), 1);
        assert_eq!(progress.pending_count(), 1);
        assert!(!progress.is_complete());
        assert!(progress.needs_resume());
    }

    #[test]
    fn undo_progress_complete() {
        let progress = UndoProgress {
            generation_from: 5,
            generation_to: 3,
            started_at: "2026-01-01".into(),
            status: UndoStatus::Completed,
            resources: HashMap::new(),
        };
        assert!(progress.is_complete());
        assert!(!progress.needs_resume());
    }

    #[test]
    fn undo_plan_serde_roundtrip() {
        let plan = sample_plan();
        let json = serde_json::to_string(&plan).unwrap();
        let parsed: UndoPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.generation_from, 12);
        assert_eq!(parsed.actions.len(), 3);
    }

    #[test]
    fn undo_progress_serde_roundtrip() {
        let mut resources = HashMap::new();
        resources.insert(
            "x".into(),
            ResourceProgress {
                status: ResourceProgressStatus::Completed,
                at: None,
            },
        );
        let progress = UndoProgress {
            generation_from: 10,
            generation_to: 8,
            started_at: "ts".into(),
            status: UndoStatus::InProgress,
            resources,
        };
        let yaml = serde_yaml_ng::to_string(&progress).unwrap();
        let parsed: UndoProgress = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.completed_count(), 1);
    }

    #[test]
    fn undo_progress_yaml_format() {
        let mut resources = HashMap::new();
        resources.insert(
            "bash-aliases".into(),
            ResourceProgress {
                status: ResourceProgressStatus::Completed,
                at: Some("2026-03-05T14:30:01Z".into()),
            },
        );
        resources.insert(
            "cargo-tools".into(),
            ResourceProgress {
                status: ResourceProgressStatus::Failed {
                    error: "SSH timeout".into(),
                },
                at: Some("2026-03-05T14:30:05Z".into()),
            },
        );
        let progress = UndoProgress {
            generation_from: 12,
            generation_to: 10,
            started_at: "2026-03-05T14:30:00Z".into(),
            status: UndoStatus::Partial,
            resources,
        };
        let yaml = serde_yaml_ng::to_string(&progress).unwrap();
        assert!(yaml.contains("partial"));
        assert!(yaml.contains("bash-aliases"));
    }
}
