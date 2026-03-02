//! Tests for FJ-1305: Purity classification and analysis.

use super::purity::{classify, level_label, recipe_purity, PurityLevel, PuritySignals};

#[test]
fn test_fj1305_pure_resource() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = classify("nginx", &signals);
    assert_eq!(result.level, PurityLevel::Pure);
    assert_eq!(result.name, "nginx");
}

#[test]
fn test_fj1305_pinned_no_sandbox() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = classify("nginx", &signals);
    assert_eq!(result.level, PurityLevel::Pinned);
    assert!(result.reasons.iter().any(|r| r.contains("no sandbox")));
}

#[test]
fn test_fj1305_pinned_no_store() {
    let signals = PuritySignals {
        has_version: true,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = classify("nginx", &signals);
    assert_eq!(result.level, PurityLevel::Pinned);
    assert!(result.reasons.iter().any(|r| r.contains("store not enabled")));
}

#[test]
fn test_fj1305_constrained_no_version() {
    let signals = PuritySignals {
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = classify("nginx", &signals);
    assert_eq!(result.level, PurityLevel::Constrained);
    assert!(result.reasons.iter().any(|r| r.contains("no version pin")));
}

#[test]
fn test_fj1305_impure_curl_pipe() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: true,
        dep_levels: vec![],
    };
    let result = classify("installer", &signals);
    assert_eq!(result.level, PurityLevel::Impure);
    assert!(result.reasons.iter().any(|r| r.contains("curl|bash")));
}

#[test]
fn test_fj1305_monotonicity_dep_elevates() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![PurityLevel::Impure],
    };
    let result = classify("derived", &signals);
    // Own level is Pure, but dep is Impure → elevated to Impure
    assert_eq!(result.level, PurityLevel::Impure);
    assert!(result.reasons.iter().any(|r| r.contains("elevates")));
}

#[test]
fn test_fj1305_monotonicity_dep_same_level() {
    let signals = PuritySignals {
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![PurityLevel::Constrained],
    };
    let result = classify("r", &signals);
    // Own = Constrained, dep = Constrained → stays Constrained
    assert_eq!(result.level, PurityLevel::Constrained);
}

#[test]
fn test_fj1305_monotonicity_dep_lower() {
    let signals = PuritySignals {
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![PurityLevel::Pure],
    };
    let result = classify("r", &signals);
    // Own = Constrained, dep = Pure → stays Constrained
    assert_eq!(result.level, PurityLevel::Constrained);
}

#[test]
fn test_fj1305_recipe_purity_max() {
    let levels = vec![PurityLevel::Pure, PurityLevel::Pinned, PurityLevel::Constrained];
    assert_eq!(recipe_purity(&levels), PurityLevel::Constrained);
}

#[test]
fn test_fj1305_recipe_purity_all_pure() {
    let levels = vec![PurityLevel::Pure, PurityLevel::Pure];
    assert_eq!(recipe_purity(&levels), PurityLevel::Pure);
}

#[test]
fn test_fj1305_recipe_purity_empty() {
    assert_eq!(recipe_purity(&[]), PurityLevel::Pure);
}

#[test]
fn test_fj1305_level_ordering() {
    assert!(PurityLevel::Pure < PurityLevel::Pinned);
    assert!(PurityLevel::Pinned < PurityLevel::Constrained);
    assert!(PurityLevel::Constrained < PurityLevel::Impure);
}

#[test]
fn test_fj1305_level_labels() {
    assert_eq!(level_label(PurityLevel::Pure), "Pure (0)");
    assert_eq!(level_label(PurityLevel::Pinned), "Pinned (1)");
    assert_eq!(level_label(PurityLevel::Constrained), "Constrained (2)");
    assert_eq!(level_label(PurityLevel::Impure), "Impure (3)");
}

#[test]
fn test_fj1305_serde_roundtrip() {
    let yaml = serde_yaml_ng::to_string(&PurityLevel::Pinned).unwrap();
    let parsed: PurityLevel = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed, PurityLevel::Pinned);
}

#[test]
fn test_fj1305_multiple_deps_max_wins() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![
            PurityLevel::Pure,
            PurityLevel::Constrained,
            PurityLevel::Pinned,
        ],
    };
    let result = classify("r", &signals);
    assert_eq!(result.level, PurityLevel::Constrained);
}

#[test]
fn test_fj1305_default_signals() {
    let signals = PuritySignals::default();
    let result = classify("bare", &signals);
    assert_eq!(result.level, PurityLevel::Constrained);
}
