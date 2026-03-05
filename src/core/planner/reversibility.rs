//! FJ-1382: Reversibility classification — classify resource operations as reversible/irreversible.

use crate::core::types::*;

/// Classification of an operation's reversibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reversibility {
    /// Operation can be undone by re-applying (e.g., create file, install package).
    Reversible,
    /// Operation cannot be undone (e.g., drop database, delete persistent volume).
    Irreversible,
}

/// Classify a planned resource operation.
pub fn classify(resource: &Resource, action: &PlanAction) -> Reversibility {
    match action {
        PlanAction::NoOp | PlanAction::Create | PlanAction::Update => Reversibility::Reversible,
        PlanAction::Destroy => classify_destroy(resource),
    }
}

/// Classify a destroy operation based on resource type and config.
fn classify_destroy(resource: &Resource) -> Reversibility {
    match resource.resource_type {
        // File destroy is reversible if source is in the config (re-createable)
        ResourceType::File => {
            if resource.content.is_some() || resource.source.is_some() {
                Reversibility::Reversible
            } else {
                Reversibility::Irreversible
            }
        }
        // Service stop is reversible (can restart)
        ResourceType::Service => Reversibility::Reversible,
        // Cron removal is reversible (can re-add)
        ResourceType::Cron => Reversibility::Reversible,
        // Package removal is reversible (can re-install)
        ResourceType::Package => Reversibility::Reversible,
        // Mount unmount is reversible
        ResourceType::Mount => Reversibility::Reversible,
        // Docker/Pepita container destroy is reversible (ephemeral)
        ResourceType::Docker | ResourceType::Pepita => Reversibility::Reversible,
        // User deletion is irreversible (home directory, data)
        ResourceType::User => Reversibility::Irreversible,
        // Network configuration destroy may lose routing state
        ResourceType::Network => Reversibility::Irreversible,
        // Model deletion loses downloaded artifacts
        ResourceType::Model => Reversibility::Irreversible,
        // GPU config changes are reversible
        ResourceType::Gpu => Reversibility::Reversible,
        // Task outputs may be irreversible
        ResourceType::Task => Reversibility::Irreversible,
        // Recipe destruction is complex — treat as irreversible
        ResourceType::Recipe => Reversibility::Irreversible,
    }
}

/// Count irreversible operations in a plan.
pub fn count_irreversible(config: &ForjarConfig, plan: &ExecutionPlan) -> usize {
    plan.changes
        .iter()
        .filter(|c| c.action == PlanAction::Destroy)
        .filter(|c| {
            config
                .resources
                .get(&c.resource_id)
                .map(|r| classify(r, &c.action) == Reversibility::Irreversible)
                .unwrap_or(true)
        })
        .count()
}

/// Format irreversible warnings for display.
pub fn warn_irreversible(config: &ForjarConfig, plan: &ExecutionPlan) -> Vec<String> {
    plan.changes
        .iter()
        .filter(|c| c.action == PlanAction::Destroy)
        .filter_map(|c| {
            let resource = config.resources.get(&c.resource_id)?;
            if classify(resource, &c.action) == Reversibility::Irreversible {
                Some(format!(
                    "{} on {} — irreversible {} destroy",
                    c.resource_id, c.machine, c.resource_type
                ))
            } else {
                None
            }
        })
        .collect()
}
