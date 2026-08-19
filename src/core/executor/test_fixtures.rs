//! Shared test helpers for executor tests.

use super::*;

pub fn local_machine() -> Machine {
    Machine {
        hostname: "localhost".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

/// A one-resource config whose file path is UNIQUE PER CALL.
///
/// It used to hardcode `/tmp/forjar-test-executor.txt`. Sixteen tests called
/// this and ten of them `remove_file`d that path in cleanup — while cargo runs
/// tests in parallel threads. They raced from the day the second one was
/// written; the race simply could not FAIL, because nothing verified that an
/// apply had produced anything. Post-apply host verification (FJ-2732) made it
/// visible immediately: three tests asserting `resources_converged == 1` for a
/// file another thread had just deleted.
///
/// Same shape as the shared-container-name races already recorded on this
/// fleet. A unique path per call removes the shared mutable global.
pub fn local_config() -> ForjarConfig {
    local_config_at(&unique_test_path())
}

/// A process- and call-unique path under /tmp, so parallel tests cannot collide.
pub fn unique_test_path() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    format!(
        "/tmp/forjar-test-executor-{}-{}.txt",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    )
}

/// Same config with an explicit path, for tests that assert on it.
pub fn local_config_at(path: &str) -> ForjarConfig {
    let yaml = format!(
        r#"
version: "1.0"
name: test
params: {{}}
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {path}
    content: "hello from forjar"
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#
    );
    serde_yaml_ng::from_str(&yaml).unwrap()
}

pub fn drift_config(file_path: &str) -> ForjarConfig {
    let yaml = format!(
        r#"
version: "1.0"
name: drift-test
params: {{}}
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {file_path}
    content: "hello from forjar"
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#
    );
    serde_yaml_ng::from_str(&yaml).unwrap()
}
