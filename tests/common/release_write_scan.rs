//! Rule 2's release-write scan (see falsification_release_workflow_shape.rs).

/// Why a workflow's text is NOT a read-only consumer of a release, if it is not.
/// Comment-only lines are ignored so prose cannot trip or hide a finding.
pub fn release_write_findings(text: &str) -> Vec<String> {
    const WRITERS: [&str; 6] = [
        "gh release create",
        "gh release upload",
        "gh release edit",
        "softprops/action-gh-release",
        "actions/upload-release-asset",
        "ncipollo/release-action",
    ];
    let mut found = Vec::new();
    for line in text.lines() {
        let code = line.split(" #").next().unwrap_or("");
        if code.trim_start().starts_with('#') {
            continue;
        }
        for w in WRITERS {
            if code.contains(w) {
                found.push(format!("`{w}`"));
            }
        }
        let squeezed: String = code.split_whitespace().collect::<Vec<_>>().join(" ");
        if squeezed.contains("contents: write") {
            found.push("`contents: write`".to_string());
        }
    }
    found.sort();
    found.dedup();
    found
}

pub fn has_v_star_tag_trigger(doc: &serde_yaml_ng::Value) -> bool {
    doc.get("on")
        .and_then(|on| on.as_mapping())
        .and_then(|m| m.get("push"))
        .and_then(|push| push.as_mapping())
        .and_then(|m| m.get("tags"))
        .and_then(|tags| tags.as_sequence())
        .is_some_and(|seq| seq.iter().any(|t| t.as_str() == Some("v*")))
}
