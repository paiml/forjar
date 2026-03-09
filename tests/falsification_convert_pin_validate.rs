//! FJ-1328/1314/1306/1329: Conversion strategy, pin tripwire, purity/repro validation.
//! Usage: cargo test --test falsification_convert_pin_validate

use forjar::core::store::convert::{analyze_conversion, ChangeType, ConversionSignals};
use forjar::core::store::lockfile::{LockFile, Pin};
use forjar::core::store::pin_tripwire::{
    check_before_apply, format_pin_report, needs_pin_update, pin_severity, PinSeverity,
};
use forjar::core::store::purity::{PurityLevel, PuritySignals};
use forjar::core::store::repro_score::ReproInput;
use forjar::core::store::validate::{
    format_purity_report, format_repro_report, validate_purity, validate_repro_score,
};
use std::collections::BTreeMap;

// ── helpers ──

fn signals(
    name: &str,
    ver: bool,
    store: bool,
    sandbox: bool,
    curl: bool,
    prov: &str,
) -> ConversionSignals {
    ConversionSignals {
        name: name.into(),
        has_version: ver,
        has_store: store,
        has_sandbox: sandbox,
        has_curl_pipe: curl,
        provider: prov.into(),
        current_version: if ver { Some("1.0".into()) } else { None },
    }
}

fn lock_file(pins: &[(&str, &str)]) -> LockFile {
    let mut map = BTreeMap::new();
    for (name, hash) in pins {
        map.insert(
            name.to_string(),
            Pin {
                provider: "apt".into(),
                version: None,
                hash: hash.to_string(),
                git_rev: None,
                pin_type: None,
            },
        );
    }
    LockFile {
        schema: "1.0".into(),
        pins: map,
    }
}

fn purity_sig(ver: bool, store: bool, sandbox: bool, curl: bool) -> PuritySignals {
    PuritySignals {
        has_version: ver,
        has_store: store,
        has_sandbox: sandbox,
        has_curl_pipe: curl,
        dep_levels: vec![],
    }
}

// ── FJ-1328: analyze_conversion ──

#[test]
fn convert_fully_pinned_resource() {
    let sigs = [signals("nginx", true, true, true, false, "apt")];
    let report = analyze_conversion(&sigs);
    assert_eq!(report.auto_change_count, 0);
    assert_eq!(report.manual_change_count, 0);
    assert_eq!(report.current_purity, PurityLevel::Pure);
    assert_eq!(report.projected_purity, PurityLevel::Pure);
}

#[test]
fn convert_unpinned_adds_version() {
    let sigs = [signals("curl", false, false, false, false, "apt")];
    let report = analyze_conversion(&sigs);
    assert!(report.resources[0]
        .auto_changes
        .iter()
        .any(|c| c.change_type == ChangeType::AddVersionPin));
}

#[test]
fn convert_no_store_adds_store_for_cacheable() {
    let sigs = [signals("curl", true, false, false, false, "apt")];
    let report = analyze_conversion(&sigs);
    assert!(report.resources[0]
        .auto_changes
        .iter()
        .any(|c| c.change_type == ChangeType::EnableStore));
}

#[test]
fn convert_no_store_adds_lock_pin() {
    let sigs = [signals("curl", true, false, false, false, "apt")];
    let report = analyze_conversion(&sigs);
    assert!(report.resources[0]
        .auto_changes
        .iter()
        .any(|c| c.change_type == ChangeType::GenerateLockPin));
}

#[test]
fn convert_curl_pipe_is_impure() {
    let sigs = [signals("installer", false, false, false, true, "apt")];
    let report = analyze_conversion(&sigs);
    assert_eq!(report.current_purity, PurityLevel::Impure);
    assert_eq!(report.projected_purity, PurityLevel::Impure);
    assert!(report.resources[0]
        .manual_changes
        .iter()
        .any(|m| m.contains("curl|bash")));
}

#[test]
fn convert_no_sandbox_manual_change() {
    let sigs = [signals("pkg", true, true, false, false, "apt")];
    let report = analyze_conversion(&sigs);
    assert!(report.resources[0]
        .manual_changes
        .iter()
        .any(|m| m.contains("sandbox")));
}

#[test]
fn convert_multiple_resources_worst_purity() {
    let sigs = [
        signals("nginx", true, true, true, false, "apt"),
        signals("curl", false, false, false, false, "apt"),
    ];
    let report = analyze_conversion(&sigs);
    assert_eq!(report.current_purity, PurityLevel::Constrained);
}

#[test]
fn convert_empty_input() {
    let report = analyze_conversion(&[]);
    assert_eq!(report.auto_change_count, 0);
    assert_eq!(report.current_purity, PurityLevel::Pure);
}

// ── FJ-1314: pin tripwire ──

#[test]
fn pin_all_fresh() {
    let lf = lock_file(&[("nginx", "blake3:abc")]);
    let mut hashes = BTreeMap::new();
    hashes.insert("nginx".into(), "blake3:abc".into());
    let result = check_before_apply(&lf, &hashes, &["nginx".into()]);
    assert!(result.all_fresh);
    assert!(result.stale_pins.is_empty());
    assert!(result.missing_inputs.is_empty());
}

#[test]
fn pin_stale_detected() {
    let lf = lock_file(&[("nginx", "blake3:old")]);
    let mut hashes = BTreeMap::new();
    hashes.insert("nginx".into(), "blake3:new".into());
    let result = check_before_apply(&lf, &hashes, &["nginx".into()]);
    assert!(!result.all_fresh);
    assert_eq!(result.stale_pins.len(), 1);
    assert_eq!(result.stale_pins[0].locked_hash, "blake3:old");
    assert_eq!(result.stale_pins[0].current_hash, "blake3:new");
}

#[test]
fn pin_missing_input() {
    let lf = lock_file(&[("nginx", "blake3:abc")]);
    let hashes = BTreeMap::new();
    let result = check_before_apply(&lf, &hashes, &["nginx".into(), "curl".into()]);
    assert!(!result.all_fresh);
    assert!(result.missing_inputs.contains(&"curl".to_string()));
}

#[test]
fn pin_severity_fresh_is_info() {
    let lf = lock_file(&[("a", "h1")]);
    let mut h = BTreeMap::new();
    h.insert("a".into(), "h1".into());
    let result = check_before_apply(&lf, &h, &["a".into()]);
    assert_eq!(pin_severity(&result, false), PinSeverity::Info);
    assert_eq!(pin_severity(&result, true), PinSeverity::Info);
}

#[test]
fn pin_severity_stale_warning() {
    let lf = lock_file(&[("a", "old")]);
    let mut h = BTreeMap::new();
    h.insert("a".into(), "new".into());
    let result = check_before_apply(&lf, &h, &["a".into()]);
    assert_eq!(pin_severity(&result, false), PinSeverity::Warning);
}

#[test]
fn pin_severity_stale_strict_error() {
    let lf = lock_file(&[("a", "old")]);
    let mut h = BTreeMap::new();
    h.insert("a".into(), "new".into());
    let result = check_before_apply(&lf, &h, &["a".into()]);
    assert_eq!(pin_severity(&result, true), PinSeverity::Error);
}

#[test]
fn pin_format_report_stale() {
    let lf = lock_file(&[("pkg", "blake3:old")]);
    let mut h = BTreeMap::new();
    h.insert("pkg".into(), "blake3:new".into());
    let result = check_before_apply(&lf, &h, &["pkg".into()]);
    let report = format_pin_report(&result);
    assert!(report.contains("STALE"));
    assert!(report.contains("blake3:old"));
}

#[test]
fn pin_format_report_missing() {
    let lf = lock_file(&[]);
    let h = BTreeMap::new();
    let result = check_before_apply(&lf, &h, &["pkg".into()]);
    let report = format_pin_report(&result);
    assert!(report.contains("MISSING"));
}

#[test]
fn pin_needs_update_stale() {
    let lf = lock_file(&[("a", "old")]);
    let mut h = BTreeMap::new();
    h.insert("a".into(), "new".into());
    assert!(needs_pin_update(&lf, &h, &["a".into()]));
}

#[test]
fn pin_needs_update_fresh() {
    let lf = lock_file(&[("a", "h1")]);
    let mut h = BTreeMap::new();
    h.insert("a".into(), "h1".into());
    assert!(!needs_pin_update(&lf, &h, &["a".into()]));
}

// ── FJ-1306/1329: validate purity ──

#[test]
fn validate_purity_pure_passes() {
    let sig = purity_sig(true, true, true, false);
    let v = validate_purity(&[("nginx", &sig)], Some(PurityLevel::Pure));
    assert!(v.pass);
    assert_eq!(v.recipe_purity, PurityLevel::Pure);
}

#[test]
fn validate_purity_constrained_fails_pure_requirement() {
    let sig = purity_sig(false, false, false, false);
    let v = validate_purity(&[("nginx", &sig)], Some(PurityLevel::Pure));
    assert!(!v.pass);
}

#[test]
fn validate_purity_no_requirement_always_passes() {
    let sig = purity_sig(false, false, false, true);
    let v = validate_purity(&[("nginx", &sig)], None);
    assert!(v.pass);
}

#[test]
fn validate_purity_report_format() {
    let sig = purity_sig(true, true, true, false);
    let v = validate_purity(&[("nginx", &sig)], Some(PurityLevel::Pure));
    let report = format_purity_report(&v);
    assert!(report.contains("PASS"));
    assert!(report.contains("nginx"));
}

#[test]
fn validate_repro_all_pure() {
    let inputs = vec![
        ReproInput {
            name: "a".into(),
            purity: PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
        ReproInput {
            name: "b".into(),
            purity: PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
    ];
    let v = validate_repro_score(&inputs, Some(90.0));
    assert!(v.pass);
    assert!(v.score.composite >= 90.0);
}

#[test]
fn validate_repro_fails_threshold() {
    let inputs = vec![ReproInput {
        name: "a".into(),
        purity: PurityLevel::Impure,
        has_store: false,
        has_lock_pin: false,
    }];
    let v = validate_repro_score(&inputs, Some(90.0));
    assert!(!v.pass);
}

#[test]
fn validate_repro_no_threshold() {
    let inputs = vec![ReproInput {
        name: "a".into(),
        purity: PurityLevel::Impure,
        has_store: false,
        has_lock_pin: false,
    }];
    let v = validate_repro_score(&inputs, None);
    assert!(v.pass);
}

#[test]
fn validate_repro_report_format() {
    let inputs = vec![ReproInput {
        name: "a".into(),
        purity: PurityLevel::Pure,
        has_store: true,
        has_lock_pin: true,
    }];
    let v = validate_repro_score(&inputs, Some(50.0));
    let report = format_repro_report(&v);
    assert!(report.contains("Reproducibility"));
    assert!(report.contains("PASS"));
    assert!(report.contains("Purity"));
}
