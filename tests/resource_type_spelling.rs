//! forjar#433 falsifier: CLI display sites print ResourceType in its serde spelling
//! (`systemd_unit`), never the Debug spelling, so the output can be typed back into `type:`.
use std::fs;
use std::path::Path;

fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in fs::read_dir(dir).expect("src/cli readable") {
        let p = e.expect("entry").path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn cli_never_debug_formats_resource_type() {
    let mut files = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cli"),
        &mut files,
    );
    assert!(
        files.len() > 50,
        "scanned only {} files: vacuous",
        files.len()
    );
    let mut hits = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).expect("utf8");
        for (i, l) in text.lines().enumerate() {
            if l.contains("{:?}") && l.contains("resource_type") {
                hits.push(format!("{}:{}: {}", f.display(), i + 1, l.trim()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "Debug-formatted ResourceType in CLI:\n{}",
        hits.join("\n")
    );
}

#[test]
fn display_is_serde_spelling() {
    use forjar::core::types::ResourceType;
    assert_eq!(ResourceType::WasmBundle.to_string(), "wasm_bundle");
    assert_eq!(ResourceType::GithubRelease.to_string(), "github_release");
}
