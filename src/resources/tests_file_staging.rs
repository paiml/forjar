//! forjar#638: the file arm stages to a per-run name, never a fixed one.

use super::file::*;
use super::tests_file::make_file_resource;

/// forjar#638: staged files carry a per-run suffix, so look for the prefix.
pub(super) fn has_staged_residue(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".forjar-new")
    })
}

/// forjar#638: the staging name is per-run and never the fixed `<path>.forjar-new`.
#[test]
fn test_638_staging_name_is_per_run() {
    let mut r = make_file_resource("/etc/conf", Some("x\n"));
    r.state = Some("file".to_string());
    let script = apply_script(&r);
    assert!(script.contains("mv -f '/etc/conf.forjar-new.'$$ '/etc/conf'"));
    assert_eq!(
        script.matches("forjar-new").count(),
        script.matches("forjar-new.'$$").count(),
        "every staging reference must carry the per-run suffix"
    );
}
