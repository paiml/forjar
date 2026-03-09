//! FJ-2003/114: Generation diffs and DO-330 tool qualification.
//!
//! Popperian rejection criteria for:
//! - FJ-2003: GenerationDiff (counts, has_changes, format_summary),
//!   ResourceDiff (builders, with_hashes, with_detail), diff_resource_sets
//!   (basic, empty, all-new, all-removed, sorted), DiffAction Display
//! - FJ-114: generate_qualification_package, ToolQualLevel Display, serde
//!
//! Usage: cargo test --test falsification_diff_do330

use forjar::core::types::{diff_resource_sets, DiffAction, GenerationDiff, ResourceDiff};

// ============================================================================
// FJ-2003: GenerationDiff
// ============================================================================

#[test]
fn gen_diff_counts() {
    let diff = GenerationDiff {
        gen_from: 1,
        gen_to: 3,
        machine: "intel".into(),
        resources: vec![
            ResourceDiff::added("a", "package"),
            ResourceDiff::modified("b", "file"),
            ResourceDiff::removed("c", "service"),
            ResourceDiff::unchanged("d", "file"),
        ],
    };
    assert_eq!(diff.added_count(), 1);
    assert_eq!(diff.modified_count(), 1);
    assert_eq!(diff.removed_count(), 1);
    assert_eq!(diff.unchanged_count(), 1);
    assert_eq!(diff.change_count(), 3);
    assert!(diff.has_changes());
}

#[test]
fn gen_diff_no_changes() {
    let diff = GenerationDiff {
        gen_from: 5,
        gen_to: 5,
        machine: "m".into(),
        resources: vec![ResourceDiff::unchanged("x", "file")],
    };
    assert!(!diff.has_changes());
}

#[test]
fn gen_diff_format_summary() {
    let diff = GenerationDiff {
        gen_from: 5,
        gen_to: 8,
        machine: "intel".into(),
        resources: vec![
            ResourceDiff::added("new-pkg", "package"),
            ResourceDiff::modified("config", "file").with_detail("content changed"),
            ResourceDiff::removed("old-svc", "service"),
        ],
    };
    let s = diff.format_summary();
    assert!(s.contains("generation 5"));
    assert!(s.contains("+ new-pkg"));
    assert!(s.contains("~ config"));
    assert!(s.contains("- old-svc"));
    assert!(s.contains("content changed"));
}

// ============================================================================
// FJ-2003: diff_resource_sets
// ============================================================================

#[test]
fn diff_sets_basic() {
    let from = vec![("a", "file", "h1"), ("b", "pkg", "h2"), ("c", "svc", "h3")];
    let to = vec![
        ("a", "file", "h1"),
        ("b", "pkg", "new"),
        ("d", "file", "h4"),
    ];
    let diffs = diff_resource_sets(&from, &to);
    assert_eq!(diffs.len(), 4);
    let a = diffs.iter().find(|d| d.resource_id == "a").unwrap();
    assert_eq!(a.action, DiffAction::Unchanged);
    let b = diffs.iter().find(|d| d.resource_id == "b").unwrap();
    assert_eq!(b.action, DiffAction::Modified);
    let c = diffs.iter().find(|d| d.resource_id == "c").unwrap();
    assert_eq!(c.action, DiffAction::Removed);
    let d = diffs.iter().find(|d| d.resource_id == "d").unwrap();
    assert_eq!(d.action, DiffAction::Added);
}

#[test]
fn diff_sets_empty() {
    assert!(diff_resource_sets(&[], &[]).is_empty());
}

#[test]
fn diff_sets_all_added() {
    let diffs = diff_resource_sets(&[], &[("a", "file", "h1")]);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].action, DiffAction::Added);
}

#[test]
fn diff_sets_all_removed() {
    let diffs = diff_resource_sets(&[("a", "file", "h1")], &[]);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].action, DiffAction::Removed);
}

#[test]
fn diff_sets_sorted() {
    let from = vec![("z", "f", "h"), ("a", "f", "h")];
    let to = vec![("z", "f", "h"), ("a", "f", "h")];
    let diffs = diff_resource_sets(&from, &to);
    assert_eq!(diffs[0].resource_id, "a");
    assert_eq!(diffs[1].resource_id, "z");
}

// ============================================================================
// FJ-114: DO-330 Qualification
// ============================================================================

#[test]
fn do330_generate_package() {
    use forjar::core::do330::{generate_qualification_package, ToolQualLevel};
    let pkg = generate_qualification_package("1.1.1", ToolQualLevel::Tql5);
    assert_eq!(pkg.tool_name, "forjar");
    assert_eq!(pkg.qualification_level, ToolQualLevel::Tql5);
    assert!(pkg.total_requirements > 0);
    assert!(pkg.qualification_complete);
}

#[test]
fn do330_tql_display() {
    use forjar::core::do330::ToolQualLevel;
    assert_eq!(ToolQualLevel::Tql5.to_string(), "TQL-5");
    assert_eq!(ToolQualLevel::Tql1.to_string(), "TQL-1");
    assert_eq!(ToolQualLevel::Tql3.to_string(), "TQL-3");
}

#[test]
fn do330_package_serde() {
    use forjar::core::do330::{generate_qualification_package, ToolQualLevel};
    let pkg = generate_qualification_package("1.0.0", ToolQualLevel::Tql4);
    let json = serde_json::to_string(&pkg).unwrap();
    assert!(json.contains("\"qualification_level\":\"Tql4\""));
    assert!(json.contains("forjar"));
}

// ============================================================================
// FJ-2003: DiffAction Display
// ============================================================================

#[test]
fn diff_action_display() {
    assert_eq!(DiffAction::Added.to_string(), "added");
    assert_eq!(DiffAction::Removed.to_string(), "removed");
    assert_eq!(DiffAction::Modified.to_string(), "modified");
    assert_eq!(DiffAction::Unchanged.to_string(), "unchanged");
}
