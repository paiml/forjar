//! forjar#996: the `--refresh` check resolves `{{secrets.*}}` through the
//! manifest's `secrets:` provider. Runs a REAL check on localhost.

use super::*;

fn fails(expected: &str) -> bool {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("fj996_refresh"), "s3cr3t-996\n").unwrap();
    let config: ForjarConfig = serde_yaml_ng::from_str(&format!(
        "version: \"1.0\"\n\
         name: refresh-secrets\n\
         secrets:\n\
         \x20 provider: file\n\
         \x20 path: \"{}\"\n\
         machines:\n\
         \x20 localhost:\n\
         \x20   hostname: localhost\n\
         \x20   addr: 127.0.0.1\n\
         resources:\n\
         \x20 guarded:\n\
         \x20   type: task\n\
         \x20   machine: localhost\n\
         \x20   command: \"true\"\n\
         \x20   completion_check: \"test '{{{{secrets.fj996_refresh}}}}' = '{expected}'\"\n",
        dir.path().display()
    ))
    .expect("fixture parses");
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: true,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
        refresh: true,
        force_tag: None,
    };
    refresh_check_fails(&cfg, &config.resources["guarded"])
}

#[test]
fn refresh_check_observes_divergence_behind_a_provider_secret() {
    // Before the fix the secret did not resolve, so the check reported
    // "could not look" (false) for a resource that HAD diverged.
    assert!(fails("another-value"));
}

#[test]
fn refresh_check_passes_when_the_provider_secret_matches() {
    assert!(!fails("s3cr3t-996"));
}
