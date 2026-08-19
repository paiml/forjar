//! GH-212: a content hash of an unmodified config must be constant.
//!
//! `plan --out` stamped the plan file with
//! `blake3(serde_yaml_ng::to_string(&config))`, and `apply --plan-file`
//! recomputed the same expression to decide whether the config had changed
//! underneath the plan. `ForjarConfig` holds several `HashMap` fields
//! (`params`, `include_provenance`, per-resource `env`), and serde serialises a
//! `HashMap` in ITERATION order, which Rust randomises per process. Measured on
//! the published 1.12.3 binary against a byte-identical `forjar.yaml`:
//!
//! ```text
//!   $ md5sum forjar.yaml                       # constant
//!   $ for i in $(seq 20); do forjar plan --out p.json; jq -r .config_hash p.json; done | sort | uniq -c
//!        12 blake3:12f5e1b8...
//!         8 blake3:afe3503a...
//!
//!   $ forjar plan --out p.json && forjar apply --plan-file p.json
//!   error: config has changed since plan was created — re-run `forjar plan`
//! ```
//!
//! A plan-file roundtrip therefore failed roughly half the time on a config
//! nobody touched, and the same hash is recorded in the audit trail as
//! `ApplyStarted.config_hash`, making the audit unreproducible.
//!
//! The fix is to hash a CANONICAL serialisation: mappings are emitted in sorted
//! key order at every depth, so the byte string depends on the config's content
//! and nothing else. Sequence order is preserved — it is meaningful (dependency
//! lists, package lists) and reordering it would hash two different configs the
//! same.

use crate::core::types::ForjarConfig;
use serde_yaml_ng::Value;

/// Recursively rewrite a YAML value with every mapping in sorted key order.
fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Mapping(m) => {
            let mut entries: Vec<(String, Value, Value)> = m
                .iter()
                .map(|(k, v)| (sort_key(k), k.clone(), canonicalize(v)))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut out = serde_yaml_ng::Mapping::with_capacity(entries.len());
            for (_, k, v) in entries {
                out.insert(k, v);
            }
            Value::Mapping(out)
        }
        Value::Sequence(s) => Value::Sequence(s.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

/// A total, deterministic ordering key for a mapping key of any YAML shape.
///
/// Keys are almost always strings; the fallback serialises so that a non-string
/// key (a number, a nested mapping) still orders deterministically instead of
/// panicking or falling back to insertion order.
fn sort_key(key: &Value) -> String {
    match key {
        Value::String(s) => s.clone(),
        other => serde_yaml_ng::to_string(other).unwrap_or_default(),
    }
}

/// Canonical YAML text for a config: identical content ⇒ identical bytes.
pub fn canonical_config_yaml(config: &ForjarConfig) -> Result<String, String> {
    let value: Value =
        serde_yaml_ng::to_value(config).map_err(|e| format!("serialize config: {e}"))?;
    serde_yaml_ng::to_string(&canonicalize(&value)).map_err(|e| format!("serialize config: {e}"))
}

/// The `blake3:`-prefixed content hash used by plan files and the audit trail.
pub fn config_hash(config: &ForjarConfig) -> Result<String, String> {
    let yaml = canonical_config_yaml(config)?;
    Ok(format!("blake3:{}", blake3::hash(yaml.as_bytes()).to_hex()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_yaml() -> &'static str {
        "version: \"1.0\"\n\
         name: repro\n\
         params:\n\
         \x20 alpha: a\n\
         \x20 bravo: b\n\
         \x20 charlie: c\n\
         \x20 delta: d\n\
         \x20 echo: e\n\
         \x20 foxtrot: f\n\
         \x20 golf: g\n\
         \x20 hotel: h\n\
         machines:\n\
         \x20 local:\n\
         \x20   hostname: localhost\n\
         \x20   addr: 127.0.0.1\n\
         \x20   user: nobody\n\
         \x20   arch: x86_64\n\
         resources:\n\
         \x20 a-file:\n\
         \x20   type: file\n\
         \x20   machine: local\n\
         \x20   path: /tmp/a.txt\n\
         \x20   content: \"aaa\\n\"\n"
    }

    fn parse() -> ForjarConfig {
        serde_yaml_ng::from_str(cfg_yaml()).expect("fixture parses")
    }

    /// RED on the shipped code: `serde_yaml_ng::to_string(&config)` over a
    /// config with 8 `params` produced several distinct hashes across
    /// processes, and more than one within a single process once the maps were
    /// rebuilt. Parsing the same text repeatedly reproduces the map-order
    /// variation that broke the plan-file roundtrip.
    #[test]
    fn a_byte_identical_config_hashes_to_one_value() {
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..200 {
            seen.insert(config_hash(&parse()).expect("hashable"));
        }
        assert_eq!(
            seen.len(),
            1,
            "config_hash is nondeterministic across {} parses of identical text: {seen:?}",
            200
        );
    }

    /// Non-regression: "deterministic" must not mean "constant". A hash that
    /// ignores the content would also pass the test above.
    #[test]
    fn a_changed_config_hashes_differently() {
        let a = config_hash(&parse()).expect("hashable");
        let mut changed = parse();
        changed.name = "other".to_string();
        let b = config_hash(&changed).expect("hashable");
        assert_ne!(a, b, "an edited config must not keep its hash");
    }

    #[test]
    fn params_are_emitted_in_sorted_order() {
        let yaml = canonical_config_yaml(&parse()).expect("hashable");
        let pos = |k: &str| yaml.find(k).unwrap_or(usize::MAX);
        assert!(pos("alpha") < pos("bravo"), "{yaml}");
        assert!(pos("bravo") < pos("charlie"), "{yaml}");
    }

    #[test]
    fn sequence_order_is_preserved() {
        // Reordering a dependency or package list is a real config change.
        let a: Value = serde_yaml_ng::from_str("k: [1, 2, 3]").expect("yaml");
        let b: Value = serde_yaml_ng::from_str("k: [3, 2, 1]").expect("yaml");
        assert_ne!(
            serde_yaml_ng::to_string(&canonicalize(&a)).ok(),
            serde_yaml_ng::to_string(&canonicalize(&b)).ok()
        );
    }

    #[test]
    fn nested_mappings_are_sorted_too() {
        let v: Value =
            serde_yaml_ng::from_str("outer:\n  z: 1\n  a:\n    zz: 1\n    aa: 2\n").expect("yaml");
        let out = serde_yaml_ng::to_string(&canonicalize(&v)).expect("yaml");
        assert!(out.find("aa").unwrap() < out.find("zz").unwrap(), "{out}");
    }

    #[test]
    fn hash_is_prefixed_like_every_other_forjar_hash() {
        assert!(config_hash(&parse())
            .expect("hashable")
            .starts_with("blake3:"));
    }
}
