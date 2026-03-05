//! FJ-2003: Stack undo — plan, progress tracking, and resume.
//!
//! ```bash
//! cargo run --example undo_plan
//! ```

use forjar::core::types::{
    ResourceProgress, ResourceProgressStatus, UndoAction, UndoPlan, UndoProgress,
    UndoResourceAction, UndoStatus,
};
use std::collections::HashMap;

fn main() {
    // Build an undo plan: generation 12 → 10
    let plan = UndoPlan {
        generation_from: 12,
        generation_to: 10,
        machines: vec!["intel".into(), "jetson".into()],
        actions: vec![
            UndoResourceAction {
                resource_id: "new-monitoring-pkg".into(),
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
            UndoResourceAction {
                resource_id: "cuda-toolkit".into(),
                machine: "jetson".into(),
                action: UndoAction::Update,
                reversible: false,
            },
        ],
        dry_run: false,
    };

    println!("=== Undo Plan ===");
    print!("{}", plan.format_summary());
    println!();

    // Simulate partial execution
    let mut resources = HashMap::new();
    resources.insert(
        "new-monitoring-pkg".into(),
        ResourceProgress {
            status: ResourceProgressStatus::Completed,
            at: Some("2026-03-05T14:30:01Z".into()),
        },
    );
    resources.insert(
        "old-config".into(),
        ResourceProgress {
            status: ResourceProgressStatus::Completed,
            at: Some("2026-03-05T14:30:02Z".into()),
        },
    );
    resources.insert(
        "bash-aliases".into(),
        ResourceProgress {
            status: ResourceProgressStatus::Failed {
                error: "SSH connection reset by peer".into(),
            },
            at: Some("2026-03-05T14:30:05Z".into()),
        },
    );
    resources.insert(
        "cuda-toolkit".into(),
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

    println!("=== Undo Progress ===");
    println!(
        "Status: {:?} ({} completed, {} failed, {} pending)",
        progress.status,
        progress.completed_count(),
        progress.failed_count(),
        progress.pending_count(),
    );
    println!("Needs resume: {}", progress.needs_resume());
    println!();

    // YAML output (matches undo-progress.yaml format)
    println!("=== Progress YAML ===");
    print!("{}", serde_yaml_ng::to_string(&progress).unwrap());
}
