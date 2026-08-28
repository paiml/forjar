//! `default-features = false` must actually subtract something.
//!
//! forjar#237. `Cargo.toml` opened a `[features]` table that never defined a
//! `default` key, and every expensive dependency was declared unconditionally.
//! Cargo's `default-features = false` removes exactly the features listed under
//! `default`; with no such key it removes the empty set. Measured on the tree
//! that filed the issue:
//!
//!   cargo tree -e normal --prefix none              | sort -u | wc -l -> 359
//!   cargo tree --no-default-features -e normal ...  | sort -u | wc -l -> 359
//!   diff -> exit 0, zero lines
//!
//! So a consumer that wanted `forjar::api` — hashing and a content store — paid
//! for an MCP server (pforge-runtime -> pmcp -> reqwest -> rustls -> aws-lc-sys,
//! 68 MB of C and assembly), a bundled SQLite compiled from source, a second TLS
//! stack through native-tls -> openssl-sys, and a multi-thread tokio runtime.
//!
//! THIS TEST READS THE MANIFEST, NOT THE TOOLCHAIN. The repo's convention for
//! cargo-adjacent tests is to never invoke the real cargo (it needs the registry
//! and is flaky offline); the expensive half of the proof — that the library
//! genuinely COMPILES with the defaults off — lives in the `no-default-features`
//! CI job, which is wired into the required `gate`.

use std::collections::BTreeSet;
use std::path::Path;

/// The dependencies the issue names as the cost of consuming forjar as a
/// library. Every one of these must be reachable ONLY through a feature.
const HEAVY: &[&str] = &[
    "clap",
    "clap_complete",
    "pforge-runtime",
    "pforge-config",
    "tokio",
    "async-trait",
    "schemars",
    "rusqlite",
    "openssl",
];

fn manifest() -> toml::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = std::fs::read_to_string(&path).expect("Cargo.toml must be readable");
    toml::from_str(&text).expect("Cargo.toml must parse as TOML")
}

fn table<'a>(v: &'a toml::Value, key: &str) -> &'a toml::Table {
    v.get(key)
        .and_then(|t| t.as_table())
        .unwrap_or_else(|| panic!("Cargo.toml must have a [{key}] table"))
}

/// Record `entry` as it appears inside a feature's array.
///
/// `dep:x` names an optional dependency; `x/y` enables feature `y` of dependency
/// `x` and, without the `?`, force-enables `x` itself; `x?/y` enables nothing on
/// its own. Anything else is another feature name.
fn absorb(
    features: &toml::Table,
    entry: &str,
    seen: &mut BTreeSet<String>,
    deps: &mut BTreeSet<String>,
) {
    if let Some(dep) = entry.strip_prefix("dep:") {
        deps.insert(dep.to_string());
    } else if entry.contains("?/") {
        // weak: enables nothing by itself
    } else if let Some((dep, _feat)) = entry.split_once('/') {
        deps.insert(dep.to_string());
    } else {
        expand(features, entry, seen, deps);
    }
}

/// Transitively enable `name`, collecting the dependency names it turns on.
///
/// A name that is not a key of `[features]` is the implicit feature of an
/// optional dependency of the same name, so it contributes that dependency.
fn expand(
    features: &toml::Table,
    name: &str,
    seen: &mut BTreeSet<String>,
    deps: &mut BTreeSet<String>,
) {
    if !seen.insert(name.to_string()) {
        return;
    }
    let Some(entries) = features.get(name).and_then(|v| v.as_array()) else {
        deps.insert(name.to_string());
        return;
    };
    for entry in entries.iter().filter_map(|v| v.as_str()) {
        absorb(features, entry, seen, deps);
    }
}

/// The dependency names enabled by turning on exactly `seeds`.
fn enabled_deps(features: &toml::Table, seeds: &[&str]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut deps = BTreeSet::new();
    for seed in seeds {
        expand(features, seed, &mut seen, &mut deps);
    }
    deps
}

/// Dependencies with no `optional = true`: present in every build, feature flags
/// notwithstanding.
fn unconditional_deps(m: &toml::Value) -> BTreeSet<String> {
    table(m, "dependencies")
        .iter()
        .filter(|(_, spec)| {
            !spec
                .get("optional")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false)
        })
        .map(|(name, _)| name.clone())
        .collect()
}

#[test]
fn default_feature_set_exists() {
    let m = manifest();
    let features = table(&m, "features");
    let default = features.get("default").unwrap_or_else(|| {
        panic!(
            "[features] has no `default` key, so `default-features = false` \
             subtracts the empty set and is a literal no-op (#237)"
        )
    });
    let entries = default
        .as_array()
        .expect("`default` must be an array of feature names");
    assert!(
        !entries.is_empty(),
        "`default` is empty, so it still subtracts nothing (#237)"
    );
}

#[test]
fn every_heavy_dep_is_optional() {
    let m = manifest();
    let unconditional = unconditional_deps(&m);
    let stuck: Vec<&&str> = HEAVY
        .iter()
        .filter(|d| unconditional.contains(**d))
        .collect();
    assert!(
        stuck.is_empty(),
        "these dependencies carry no `optional = true`, so no feature flag can \
         remove them: {stuck:?} (#237)"
    );
}

#[test]
fn default_still_enables_everything_the_binary_needs() {
    let m = manifest();
    let features = table(&m, "features");
    assert!(
        features.contains_key("default"),
        "no `default` key to check (#237)"
    );
    let enabled = enabled_deps(features, &["default"]);
    let missing: Vec<&&str> = HEAVY.iter().filter(|d| !enabled.contains(**d)).collect();
    assert!(
        missing.is_empty(),
        "the DEFAULT build would ship without {missing:?}. Marking a dependency \
         optional and forgetting it in the `default` closure still compiles and \
         still produces a binary — one missing a subcommand."
    );
}

#[test]
fn no_default_features_drops_the_heavy_set() {
    let m = manifest();
    let required = unconditional_deps(&m);
    let survivors: Vec<&&str> = HEAVY.iter().filter(|d| required.contains(**d)).collect();
    assert!(
        survivors.is_empty(),
        "`default-features = false` still drags in {survivors:?} (#237)"
    );
    // Anti-vacuity: the trimmed library is a real crate, not an empty one.
    assert!(
        required.contains("blake3") && required.contains("serde"),
        "the no-default build lost dependencies the library itself needs; \
         `unconditional_deps` is measuring the wrong thing"
    );
}

#[test]
fn the_binary_is_gated_on_cli() {
    let m = manifest();
    let bins = m.get("bin").and_then(|b| b.as_array()).unwrap_or_else(|| {
        panic!(
            "no [[bin]] section: src/main.rs is auto-discovered and would be \
                 compiled with `cli` off, where `clap::Parser` and \
                 `forjar::cli::Commands` do not exist (#237)"
        )
    });
    let forjar = bins
        .iter()
        .find(|b| b.get("name").and_then(toml::Value::as_str) == Some("forjar"))
        .expect("[[bin]] must declare the `forjar` binary");
    let required: Vec<&str> = forjar
        .get("required-features")
        .and_then(|r| r.as_array())
        .map(|a| a.iter().filter_map(toml::Value::as_str).collect())
        .unwrap_or_default();
    assert!(
        required.contains(&"cli"),
        "[[bin]] forjar must carry `required-features = [\"cli\"]`, found {required:?}"
    );
}

#[test]
fn vendored_openssl_still_reaches_openssl() {
    let m = manifest();
    let features = table(&m, "features");
    let enabled = enabled_deps(features, &["vendored-openssl"]);
    assert!(
        enabled.contains("openssl"),
        "`vendored-openssl` no longer enables the openssl dependency. Every \
         cross-compiled release artifact is built with `--features \
         vendored-openssl` and no `--no-default-features`, so this breaks at tag \
         time, on cross, for musl and darwin only."
    );
}
