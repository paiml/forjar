//! FJ-1328: Recipe conversion strategy.
//!
//! Automates the 5-step conversion ladder for making recipes reproducible:
//! 1. Add version pins to all packages
//! 2. Add `store: true` to cacheable resources
//! 3. Generate `forjar.inputs.lock.yaml`
//! 4. Add `sandbox:` blocks (manual step — reported only)
//! 5. Replace imperative hooks (manual step — reported only)

use super::purity::PurityLevel;
use serde::{Deserialize, Serialize};

/// A single resource's conversion analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceConversion {
    /// Resource name
    pub name: String,

    /// Provider (apt, cargo, nix, etc.)
    pub provider: String,

    /// Current purity level
    pub current_purity: PurityLevel,

    /// Target purity level after automated steps
    pub target_purity: PurityLevel,

    /// Automated changes to apply
    pub auto_changes: Vec<ConversionChange>,

    /// Manual changes required (reported but not applied)
    pub manual_changes: Vec<String>,
}

/// A single automated change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversionChange {
    /// Type of change
    pub change_type: ChangeType,
    /// Human-readable description
    pub description: String,
}

/// Types of automated conversion changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Add version pin (step 1)
    AddVersionPin,
    /// Enable store (step 2)
    EnableStore,
    /// Generate lock file entry (step 3)
    GenerateLockPin,
}

/// Signals from a recipe resource for conversion analysis.
#[derive(Debug, Clone)]
pub struct ConversionSignals {
    pub name: String,
    pub has_version: bool,
    pub has_store: bool,
    pub has_sandbox: bool,
    pub has_curl_pipe: bool,
    pub provider: String,
    pub current_version: Option<String>,
}

/// Overall conversion report for a recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversionReport {
    /// Resource conversions
    pub resources: Vec<ResourceConversion>,

    /// Count of automated changes
    pub auto_change_count: usize,

    /// Count of manual changes needed
    pub manual_change_count: usize,

    /// Current overall purity
    pub current_purity: PurityLevel,

    /// Projected purity after automated steps
    pub projected_purity: PurityLevel,
}

/// Analyze a set of resources and produce conversion recommendations.
pub fn analyze_conversion(signals: &[ConversionSignals]) -> ConversionReport {
    let mut resources = Vec::new();
    let mut auto_count = 0;
    let mut manual_count = 0;

    for sig in signals {
        let conv = analyze_resource(sig);
        auto_count += conv.auto_changes.len();
        manual_count += conv.manual_changes.len();
        resources.push(conv);
    }

    let current_levels: Vec<PurityLevel> = resources.iter().map(|r| r.current_purity).collect();
    let projected_levels: Vec<PurityLevel> = resources.iter().map(|r| r.target_purity).collect();

    let current_purity = worst_purity(&current_levels);
    let projected_purity = worst_purity(&projected_levels);

    ConversionReport {
        resources,
        auto_change_count: auto_count,
        manual_change_count: manual_count,
        current_purity,
        projected_purity,
    }
}

fn analyze_resource(sig: &ConversionSignals) -> ResourceConversion {
    let mut auto_changes = Vec::new();
    let mut manual_changes = Vec::new();

    // Step 1: Version pin
    if !sig.has_version {
        auto_changes.push(ConversionChange {
            change_type: ChangeType::AddVersionPin,
            description: format!("Add version pin to {} ({})", sig.name, sig.provider),
        });
    }

    // Step 2: Enable store
    if !sig.has_store && is_cacheable_provider(&sig.provider) {
        auto_changes.push(ConversionChange {
            change_type: ChangeType::EnableStore,
            description: format!("Add store: true to {}", sig.name),
        });
    }

    // Step 3: Lock file pin
    if !sig.has_store {
        auto_changes.push(ConversionChange {
            change_type: ChangeType::GenerateLockPin,
            description: format!("Generate lock file entry for {}", sig.name),
        });
    }

    // Step 4: Sandbox (manual)
    if !sig.has_sandbox && !sig.has_curl_pipe {
        manual_changes.push(format!(
            "Add sandbox: block to {} for full purity",
            sig.name
        ));
    }

    // Step 5: Replace curl|bash (manual)
    if sig.has_curl_pipe {
        manual_changes.push(format!(
            "Replace curl|bash pattern in {} with declarative resource",
            sig.name
        ));
    }

    let current = classify_purity(sig);
    let target = projected_purity_after_auto(sig, &auto_changes);

    ResourceConversion {
        name: sig.name.clone(),
        provider: sig.provider.clone(),
        current_purity: current,
        target_purity: target,
        auto_changes,
        manual_changes,
    }
}

fn classify_purity(sig: &ConversionSignals) -> PurityLevel {
    if sig.has_curl_pipe {
        return PurityLevel::Impure;
    }
    if sig.has_version && sig.has_store && sig.has_sandbox {
        return PurityLevel::Pure;
    }
    if sig.has_version && sig.has_store {
        return PurityLevel::Pinned;
    }
    PurityLevel::Constrained
}

fn projected_purity_after_auto(
    sig: &ConversionSignals,
    changes: &[ConversionChange],
) -> PurityLevel {
    if sig.has_curl_pipe {
        return PurityLevel::Impure;
    }

    let will_have_version = sig.has_version
        || changes
            .iter()
            .any(|c| c.change_type == ChangeType::AddVersionPin);
    let will_have_store = sig.has_store
        || changes
            .iter()
            .any(|c| c.change_type == ChangeType::EnableStore);

    if will_have_version && will_have_store && sig.has_sandbox {
        PurityLevel::Pure
    } else if will_have_version && will_have_store {
        PurityLevel::Pinned
    } else {
        PurityLevel::Constrained
    }
}

fn is_cacheable_provider(provider: &str) -> bool {
    matches!(provider, "apt" | "cargo" | "uv" | "nix" | "docker" | "pip")
}

fn worst_purity(levels: &[PurityLevel]) -> PurityLevel {
    levels
        .iter()
        .max_by_key(|l| match l {
            PurityLevel::Pure => 0,
            PurityLevel::Pinned => 1,
            PurityLevel::Constrained => 2,
            PurityLevel::Impure => 3,
        })
        .copied()
        .unwrap_or(PurityLevel::Pure)
}
