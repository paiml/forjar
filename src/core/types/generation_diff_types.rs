//! FJ-2003: Generation diff types — cross-generation resource comparison.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2003: Cross-generation diff result.
///
/// Compares two generations and produces a list of per-resource changes.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{GenerationDiff, ResourceDiff, DiffAction};
///
/// let diff = GenerationDiff {
///     gen_from: 5,
///     gen_to: 8,
///     machine: "intel".into(),
///     resources: vec![
///         ResourceDiff::added("new-pkg", "package"),
///         ResourceDiff::modified("bash-aliases", "file"),
///         ResourceDiff::removed("old-config", "file"),
///     ],
/// };
/// assert_eq!(diff.added_count(), 1);
/// assert_eq!(diff.modified_count(), 1);
/// assert_eq!(diff.removed_count(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationDiff {
    /// Source generation number.
    pub gen_from: u32,
    /// Target generation number.
    pub gen_to: u32,
    /// Machine name.
    pub machine: String,
    /// Per-resource diffs.
    pub resources: Vec<ResourceDiff>,
}

impl GenerationDiff {
    /// Number of added resources.
    pub fn added_count(&self) -> usize {
        self.resources
            .iter()
            .filter(|r| r.action == DiffAction::Added)
            .count()
    }

    /// Number of modified resources.
    pub fn modified_count(&self) -> usize {
        self.resources
            .iter()
            .filter(|r| r.action == DiffAction::Modified)
            .count()
    }

    /// Number of removed resources.
    pub fn removed_count(&self) -> usize {
        self.resources
            .iter()
            .filter(|r| r.action == DiffAction::Removed)
            .count()
    }

    /// Number of unchanged resources.
    pub fn unchanged_count(&self) -> usize {
        self.resources
            .iter()
            .filter(|r| r.action == DiffAction::Unchanged)
            .count()
    }

    /// Total number of changes (added + modified + removed).
    pub fn change_count(&self) -> usize {
        self.added_count() + self.modified_count() + self.removed_count()
    }

    /// Whether there are any changes.
    pub fn has_changes(&self) -> bool {
        self.change_count() > 0
    }

    /// Format as a human-readable diff summary.
    pub fn format_summary(&self) -> String {
        let mut out = format!(
            "Diff: generation {} → {} ({})\n",
            self.gen_from, self.gen_to, self.machine,
        );
        out.push_str(&format!(
            "{} added, {} modified, {} removed, {} unchanged\n",
            self.added_count(),
            self.modified_count(),
            self.removed_count(),
            self.unchanged_count(),
        ));
        for r in &self.resources {
            if r.action == DiffAction::Unchanged {
                continue;
            }
            let symbol = match r.action {
                DiffAction::Added => "+",
                DiffAction::Removed => "-",
                DiffAction::Modified => "~",
                DiffAction::Unchanged => " ",
            };
            out.push_str(&format!(
                "  {symbol} {} ({})",
                r.resource_id, r.resource_type,
            ));
            if let Some(ref detail) = r.detail {
                out.push_str(&format!(" — {detail}"));
            }
            out.push('\n');
        }
        out
    }
}

/// FJ-2003: Per-resource diff entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDiff {
    /// Resource identifier.
    pub resource_id: String,
    /// Resource type (file, package, service, etc.).
    pub resource_type: String,
    /// Diff action.
    pub action: DiffAction,
    /// Old hash (in gen_from), if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_hash: Option<String>,
    /// New hash (in gen_to), if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_hash: Option<String>,
    /// Human-readable detail (e.g., "content changed", "permissions 0644 → 0600").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ResourceDiff {
    /// Create an "added" diff entry.
    pub fn added(resource_id: &str, resource_type: &str) -> Self {
        Self {
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            action: DiffAction::Added,
            old_hash: None,
            new_hash: None,
            detail: None,
        }
    }

    /// Create a "removed" diff entry.
    pub fn removed(resource_id: &str, resource_type: &str) -> Self {
        Self {
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            action: DiffAction::Removed,
            old_hash: None,
            new_hash: None,
            detail: None,
        }
    }

    /// Create a "modified" diff entry.
    pub fn modified(resource_id: &str, resource_type: &str) -> Self {
        Self {
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            action: DiffAction::Modified,
            old_hash: None,
            new_hash: None,
            detail: None,
        }
    }

    /// Create an "unchanged" diff entry.
    pub fn unchanged(resource_id: &str, resource_type: &str) -> Self {
        Self {
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            action: DiffAction::Unchanged,
            old_hash: None,
            new_hash: None,
            detail: None,
        }
    }

    /// Add hash information.
    pub fn with_hashes(mut self, old: Option<String>, new: Option<String>) -> Self {
        self.old_hash = old;
        self.new_hash = new;
        self
    }

    /// Add detail text.
    pub fn with_detail(mut self, detail: &str) -> Self {
        self.detail = Some(detail.to_string());
        self
    }
}

/// FJ-2003: Diff action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffAction {
    /// Resource exists in gen_to but not gen_from.
    Added,
    /// Resource exists in gen_from but not gen_to.
    Removed,
    /// Resource exists in both but hash changed.
    Modified,
    /// Resource exists in both with same hash.
    Unchanged,
}

impl fmt::Display for DiffAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Added => write!(f, "added"),
            Self::Removed => write!(f, "removed"),
            Self::Modified => write!(f, "modified"),
            Self::Unchanged => write!(f, "unchanged"),
        }
    }
}

/// FJ-2003: Compute diff between two resource sets (keyed by resource_id).
///
/// Each entry is `(resource_id, resource_type, hash)`.
pub fn diff_resource_sets(
    from: &[(&str, &str, &str)],
    to: &[(&str, &str, &str)],
) -> Vec<ResourceDiff> {
    use std::collections::HashMap;

    let from_map: HashMap<&str, (&str, &str)> =
        from.iter().map(|(id, ty, h)| (*id, (*ty, *h))).collect();
    let to_map: HashMap<&str, (&str, &str)> =
        to.iter().map(|(id, ty, h)| (*id, (*ty, *h))).collect();

    let mut diffs = Vec::new();

    // Check resources in `to` (added or modified)
    for (id, (ty, new_hash)) in &to_map {
        if let Some((_, old_hash)) = from_map.get(id) {
            if old_hash == new_hash {
                diffs.push(ResourceDiff::unchanged(id, ty));
            } else {
                diffs.push(
                    ResourceDiff::modified(id, ty)
                        .with_hashes(Some(old_hash.to_string()), Some(new_hash.to_string()))
                        .with_detail("hash changed"),
                );
            }
        } else {
            diffs.push(ResourceDiff::added(id, ty));
        }
    }

    // Check resources only in `from` (removed)
    for (id, (ty, _)) in &from_map {
        if !to_map.contains_key(id) {
            diffs.push(ResourceDiff::removed(id, ty));
        }
    }

    diffs.sort_by(|a, b| a.resource_id.cmp(&b.resource_id));
    diffs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_action_display() {
        assert_eq!(DiffAction::Added.to_string(), "added");
        assert_eq!(DiffAction::Removed.to_string(), "removed");
        assert_eq!(DiffAction::Modified.to_string(), "modified");
        assert_eq!(DiffAction::Unchanged.to_string(), "unchanged");
    }

    #[test]
    fn resource_diff_builders() {
        let a = ResourceDiff::added("pkg", "package");
        assert_eq!(a.action, DiffAction::Added);
        assert!(a.old_hash.is_none());

        let m = ResourceDiff::modified("cfg", "file")
            .with_hashes(Some("old".into()), Some("new".into()))
            .with_detail("content changed");
        assert_eq!(m.action, DiffAction::Modified);
        assert_eq!(m.old_hash.as_deref(), Some("old"));
        assert_eq!(m.detail.as_deref(), Some("content changed"));
    }

    #[test]
    fn generation_diff_counts() {
        let diff = GenerationDiff {
            gen_from: 1,
            gen_to: 3,
            machine: "m".into(),
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
    fn generation_diff_no_changes() {
        let diff = GenerationDiff {
            gen_from: 1,
            gen_to: 1,
            machine: "m".into(),
            resources: vec![ResourceDiff::unchanged("x", "file")],
        };
        assert!(!diff.has_changes());
        assert_eq!(diff.change_count(), 0);
    }

    #[test]
    fn generation_diff_format() {
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
        assert!(s.contains("generation 5 → 8"));
        assert!(s.contains("intel"));
        assert!(s.contains("+ new-pkg"));
        assert!(s.contains("~ config"));
        assert!(s.contains("- old-svc"));
        assert!(s.contains("content changed"));
    }

    #[test]
    fn diff_resource_sets_basic() {
        let from = vec![
            ("a", "file", "h1"),
            ("b", "package", "h2"),
            ("c", "service", "h3"),
        ];
        let to = vec![
            ("a", "file", "h1"),       // unchanged
            ("b", "package", "h2new"), // modified
            ("d", "file", "h4"),       // added
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
    fn diff_resource_sets_empty() {
        let diffs = diff_resource_sets(&[], &[]);
        assert!(diffs.is_empty());
    }

    #[test]
    fn diff_resource_sets_all_new() {
        let diffs = diff_resource_sets(&[], &[("a", "file", "h1")]);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].action, DiffAction::Added);
    }

    #[test]
    fn diff_resource_sets_all_removed() {
        let diffs = diff_resource_sets(&[("a", "file", "h1")], &[]);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].action, DiffAction::Removed);
    }

    #[test]
    fn diff_resource_sets_sorted() {
        let from = vec![("z", "file", "h1"), ("a", "file", "h2")];
        let to = vec![("z", "file", "h1"), ("a", "file", "h2")];
        let diffs = diff_resource_sets(&from, &to);
        assert_eq!(diffs[0].resource_id, "a");
        assert_eq!(diffs[1].resource_id, "z");
    }

    #[test]
    fn generation_diff_serde_roundtrip() {
        let diff = GenerationDiff {
            gen_from: 1,
            gen_to: 3,
            machine: "m".into(),
            resources: vec![
                ResourceDiff::added("a", "file"),
                ResourceDiff::modified("b", "pkg").with_detail("hash changed"),
            ],
        };
        let json = serde_json::to_string(&diff).unwrap();
        let parsed: GenerationDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.gen_from, 1);
        assert_eq!(parsed.resources.len(), 2);
    }

    #[test]
    fn diff_action_serde() {
        for action in [
            DiffAction::Added,
            DiffAction::Removed,
            DiffAction::Modified,
            DiffAction::Unchanged,
        ] {
            let json = serde_json::to_string(&action).unwrap();
            let parsed: DiffAction = serde_json::from_str(&json).unwrap();
            assert_eq!(action, parsed);
        }
    }
}
