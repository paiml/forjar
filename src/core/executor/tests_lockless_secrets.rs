//! forjar#996: the lockless check resolves `{{secrets.*}}` through the
//! manifest's `secrets:` provider. Runs a REAL check on localhost.

use super::host_check_passes;
use crate::core::types::ForjarConfig;

fn cfg(dir: &std::path::Path, expected: &str) -> ForjarConfig {
    std::fs::write(dir.join("fj996_lockless"), "s3cr3t-996\n").unwrap();
    serde_yaml_ng::from_str(&format!(
        "version: \"1.0\"\n\
         name: lockless-secrets\n\
         secrets:\n\
         \x20 provider: file\n\
         \x20 path: \"{}\"\n\
         machines:\n\
         \x20 local:\n\
         \x20   hostname: localhost\n\
         \x20   addr: localhost\n\
         resources:\n\
         \x20 guarded:\n\
         \x20   type: task\n\
         \x20   machine: local\n\
         \x20   command: \"true\"\n\
         \x20   completion_check: \"test '{{{{secrets.fj996_lockless}}}}' = '{expected}'\"\n",
        dir.display()
    ))
    .expect("fixture parses")
}

#[test]
fn lockless_check_reads_the_secret_from_the_declared_provider() {
    let dir = tempfile::tempdir().unwrap();
    let config = cfg(dir.path(), "s3cr3t-996");
    assert!(host_check_passes(
        &config,
        &config.resources["guarded"],
        "local"
    ));
}

#[test]
fn lockless_check_still_fails_when_the_resolved_value_differs() {
    let dir = tempfile::tempdir().unwrap();
    let config = cfg(dir.path(), "another-value");
    assert!(!host_check_passes(
        &config,
        &config.resources["guarded"],
        "local"
    ));
}
