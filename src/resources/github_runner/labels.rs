//! forjar#1000: comparing the labels and group a manifest declares with what
//! GitHub lists for the seat.
//!
//! GitHub adds read-only labels itself (`self-hosted`, the OS and the
//! architecture). A manifest may name them or not, so they are handled apart:
//!
//! * the CUSTOM labels must match exactly, as a set, in any order;
//! * every IMPLICIT label the manifest names must be present on the seat.
//!
//! Comparison is case-insensitive (GitHub lists `Linux`, `X64`); messages show
//! the names as given.

use std::collections::BTreeMap;

/// Labels GitHub assigns on its own; never passed to `config.sh --labels`.
const IMPLICIT: [&str; 7] = [
    "self-hosted",
    "linux",
    "windows",
    "macos",
    "x64",
    "arm",
    "arm64",
];

/// What GitHub lists for one label: its name and whether it is `read-only`.
pub(super) struct ListedLabel {
    pub name: String,
    pub read_only: bool,
}

pub(super) fn is_implicit(label: &str) -> bool {
    IMPLICIT.contains(&label.to_ascii_lowercase().as_str())
}

/// The labels `config.sh --labels` must be given: the custom ones, in the
/// order declared, without duplicates.
pub(super) fn custom_for_config(declared: &[String]) -> Vec<String> {
    let mut seen = Vec::<String>::new();
    for l in declared.iter().filter(|l| !is_implicit(l)) {
        if !seen.iter().any(|s| s.eq_ignore_ascii_case(l)) {
            seen.push(l.clone());
        }
    }
    seen
}

fn keyed<'a>(names: impl Iterator<Item = &'a str>) -> BTreeMap<String, String> {
    names
        .map(|n| (n.to_ascii_lowercase(), n.to_string()))
        .collect()
}

fn show(names: &BTreeMap<String, String>) -> String {
    let v: Vec<&str> = names.values().map(String::as_str).collect();
    format!("[{}]", v.join(", "))
}

fn minus(a: &BTreeMap<String, String>, b: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    a.iter()
        .filter(|(k, _)| !b.contains_key(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// `None` when the declared labels and group agree with the listing; otherwise
/// the difference, naming both sets (and both groups when the group differs).
pub(super) fn difference(
    declared: &[String],
    listed: &[ListedLabel],
    declared_group: Option<&str>,
    listed_group: Option<&str>,
) -> Option<String> {
    let want_custom = keyed(
        declared
            .iter()
            .filter(|l| !is_implicit(l))
            .map(String::as_str),
    );
    let want_implicit = keyed(
        declared
            .iter()
            .filter(|l| is_implicit(l))
            .map(String::as_str),
    );
    let have_custom = keyed(
        listed
            .iter()
            .filter(|l| !l.read_only)
            .map(|l| l.name.as_str()),
    );
    let have_all = keyed(listed.iter().map(|l| l.name.as_str()));
    let missing = {
        let mut m = minus(&want_custom, &have_custom);
        m.extend(minus(&want_implicit, &have_all));
        m
    };
    let extra = minus(&have_custom, &want_custom);
    let mut parts = Vec::new();
    if !missing.is_empty() || !extra.is_empty() {
        parts.push(format!(
            "labels differ: declared {}, listed {}; missing on the seat: {}; extra on the seat: {}",
            show(&keyed(declared.iter().map(String::as_str))),
            show(&keyed(listed.iter().map(|l| l.name.as_str()))),
            show(&missing),
            show(&extra),
        ));
    }
    if let (Some(want), Some(have)) = (declared_group, listed_group) {
        if !want.eq_ignore_ascii_case(have) {
            parts.push(format!("group differs: declared `{want}`, listed `{have}`"));
        }
    }
    (!parts.is_empty()).then(|| parts.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| (*x).to_string()).collect()
    }

    fn listed(custom: &[&str]) -> Vec<ListedLabel> {
        let mut v: Vec<ListedLabel> = ["self-hosted", "Linux", "X64"]
            .iter()
            .map(|n| ListedLabel {
                name: (*n).to_string(),
                read_only: true,
            })
            .collect();
        v.extend(custom.iter().map(|n| ListedLabel {
            name: (*n).to_string(),
            read_only: false,
        }));
        v
    }

    #[test]
    fn order_and_case_do_not_matter() {
        let d = s(&["X64", "gate", "self-hosted", "Fast", "linux"]);
        assert!(difference(&d, &listed(&["fast", "GATE"]), None, None).is_none());
    }

    #[test]
    fn implicit_labels_may_be_omitted_from_the_manifest() {
        assert!(difference(&s(&["gate"]), &listed(&["gate"]), None, None).is_none());
    }

    #[test]
    fn a_missing_custom_label_is_named() {
        let d = difference(
            &s(&["linux", "gate", "fast"]),
            &listed(&["fast"]),
            None,
            None,
        )
        .unwrap();
        assert!(d.contains("missing on the seat: [gate]"), "{d}");
        assert!(d.contains("declared [fast, gate, linux]"), "{d}");
    }

    #[test]
    fn an_extra_custom_label_is_drift() {
        let d = difference(&s(&["gate"]), &listed(&["gate", "old"]), None, None).unwrap();
        assert!(d.contains("extra on the seat: [old]"), "{d}");
    }

    #[test]
    fn an_implicit_label_the_seat_lacks_is_drift() {
        let d = difference(&s(&["arm64", "gate"]), &listed(&["gate"]), None, None).unwrap();
        assert!(d.contains("missing on the seat: [arm64]"), "{d}");
    }

    #[test]
    fn groups_compare_case_insensitively_and_name_both() {
        assert!(difference(
            &s(&["a"]),
            &listed(&["a"]),
            Some("default"),
            Some("Default")
        )
        .is_none());
        let d = difference(&s(&["a"]), &listed(&["a"]), Some("default"), Some("Other")).unwrap();
        assert!(d.contains("declared `default`, listed `Other`"), "{d}");
    }

    #[test]
    fn config_labels_drop_implicit_and_duplicates() {
        let got = custom_for_config(&s(&["self-hosted", "gate", "Gate", "linux", "fast"]));
        assert_eq!(got, s(&["gate", "fast"]));
    }
}
