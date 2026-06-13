//! COV-1 (PMAT-088): coverage for `undo::preflight_ssh_check`.
//!
//! Exercises the branch logic of the FJ-2003 multi-machine pre-flight SSH
//! check without standing up a real SSH server:
//!   * local / container machines are skipped (deterministic Ok),
//!   * an empty/filtered machine set yields Ok,
//!   * a remote machine that cannot resolve (`.invalid`, RFC 6761) is
//!     reported unreachable and yields Err — ssh fails fast with no DNS or
//!     network round-trip, so the test is hermetic and quick.

#[cfg(test)]
mod tests {
    use crate::core::types;
    use indexmap::IndexMap;

    fn machine(addr: &str, transport: Option<&str>) -> types::Machine {
        types::Machine {
            hostname: "h".to_string(),
            addr: addr.to_string(),
            user: "deploy".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: transport.map(|s| s.to_string()),
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        }
    }

    fn config_with(machines: Vec<(&str, types::Machine)>) -> types::ForjarConfig {
        let mut map = IndexMap::new();
        for (name, m) in machines {
            map.insert(name.to_string(), m);
        }
        types::ForjarConfig {
            machines: map,
            ..Default::default()
        }
    }

    #[test]
    fn preflight_ok_when_no_machines() {
        let cfg = config_with(vec![]);
        assert!(super::super::undo::preflight_ssh_check(&cfg, None).is_ok());
    }

    #[test]
    fn preflight_skips_localhost() {
        let cfg = config_with(vec![("web", machine("localhost", None))]);
        assert!(super::super::undo::preflight_ssh_check(&cfg, None).is_ok());
    }

    #[test]
    fn preflight_skips_loopback_ip() {
        let cfg = config_with(vec![("web", machine("127.0.0.1", None))]);
        assert!(super::super::undo::preflight_ssh_check(&cfg, None).is_ok());
    }

    #[test]
    fn preflight_skips_local_transport() {
        // Non-loopback addr but transport: local => skipped.
        let cfg = config_with(vec![("web", machine("10.0.0.5", Some("local")))]);
        assert!(super::super::undo::preflight_ssh_check(&cfg, None).is_ok());
    }

    #[test]
    fn preflight_skips_container_transport() {
        let cfg = config_with(vec![("ct", machine("10.0.0.6", Some("container")))]);
        assert!(super::super::undo::preflight_ssh_check(&cfg, None).is_ok());
    }

    #[test]
    fn preflight_errors_on_unreachable_remote() {
        let cfg = config_with(vec![(
            "remote",
            machine("forjar-cov-preflight.invalid", None),
        )]);
        let result = super::super::undo::preflight_ssh_check(&cfg, None);
        assert!(result.is_err(), "unreachable remote must Err: {result:?}");
        let msg = result.unwrap_err();
        assert!(msg.contains("pre-flight failed"), "msg: {msg}");
        assert!(msg.contains("remote"), "should name the machine: {msg}");
    }

    #[test]
    fn preflight_filter_skips_unreachable_remote() {
        // Filter selects only the local machine, so the unreachable remote is
        // never probed and the check passes.
        let cfg = config_with(vec![
            ("local", machine("localhost", None)),
            ("remote", machine("forjar-cov-preflight.invalid", None)),
        ]);
        let result = super::super::undo::preflight_ssh_check(&cfg, Some("local"));
        assert!(result.is_ok(), "filtered-out remote must not fail: {result:?}");
    }

    #[test]
    fn preflight_filter_selects_unreachable_remote() {
        // Filter selects the unreachable remote, so the check fails fast.
        let cfg = config_with(vec![
            ("local", machine("localhost", None)),
            ("remote", machine("forjar-cov-preflight.invalid", None)),
        ]);
        let result = super::super::undo::preflight_ssh_check(&cfg, Some("remote"));
        assert!(result.is_err(), "selected unreachable remote must Err: {result:?}");
    }
}
