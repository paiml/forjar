//! forjar#404 (CRUX audit E02): open the run's ControlMasters BEFORE the gates.
//!
//! `apply` did its remote work in two phases and multiplexed only the second.
//! The pre-apply drift gate queried every locked resource of every target
//! machine while `build_ssh_args` could still find no socket
//! (`transport/ssh.rs`), so each query was a full handshake — 306 ms median
//! measured, against 6.7 ms once the master exists. `apply_machine` then opened
//! the master twenty frames later for the convergence work.
//!
//! This module decides which machines the run will talk to and hands back a
//! guard that keeps their sockets open for the whole invocation.

use crate::core::types;
use crate::transport;

/// Should this run pay for a ControlMaster before the preflight?
///
/// Pure, so the decision is testable without a network.
///
/// A real apply always converges through the transport, so the master pays for
/// itself. A `--dry-run` executes nothing (`executor::apply` returns before it
/// reaches a machine) — but the DRIFT GATE still runs against live hosts, and
/// that is the phase this issue is about. So a dry run multiplexes exactly when
/// the gate will run, and otherwise opens nothing at all.
pub(super) fn should_multiplex(config: &types::ForjarConfig, force: bool, dry_run: bool) -> bool {
    if !dry_run {
        return true;
    }
    config.policy.tripwire && !force
}

/// The SSH-transport machines this run is scoped to.
///
/// Local, container and pepita targets are excluded by `is_ssh_transport` —
/// there is no handshake to amortise for them.
pub(super) fn ssh_machines_in_scope(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
) -> Vec<types::Machine> {
    config
        .machines
        .iter()
        .filter(|(name, _)| machine_filter.is_none_or(|f| f == name.as_str()))
        .map(|(_, m)| m)
        .filter(|m| transport::is_ssh_transport(m))
        .cloned()
        .collect()
}

/// Open the run's ControlMasters, or `None` when there is nothing to multiplex.
///
/// The returned guard must be held for the rest of `cmd_apply` — dropping it
/// early closes the sockets the gate and the executor are about to use.
#[must_use = "dropping the guard closes the sockets the apply is about to use"]
pub(super) fn open_control_masters(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> Option<transport::ssh_mux::ControlMasterGuard> {
    if !should_multiplex(config, force, dry_run) {
        return None;
    }
    let machines = ssh_machines_in_scope(config, machine_filter);
    if machines.is_empty() {
        return None;
    }
    let guard = transport::ssh_mux::ControlMasterGuard::open(&machines, verbose);
    if verbose {
        eprintln!(
            "  ssh: {} ControlMaster session(s) opened for {} machine(s)",
            guard.opened(),
            machines.len()
        );
    }
    Some(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn machine(addr: &str) -> types::Machine {
        types::Machine {
            hostname: "h".to_string(),
            addr: addr.to_string(),
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

    fn config(machines: &[(&str, &str)]) -> types::ForjarConfig {
        let mut m = IndexMap::new();
        for (name, addr) in machines {
            m.insert((*name).to_string(), machine(addr));
        }
        types::ForjarConfig {
            machines: m,
            ..Default::default()
        }
    }

    #[test]
    fn a_real_apply_always_multiplexes() {
        let mut c = config(&[]);
        c.policy.tripwire = false;
        assert!(should_multiplex(&c, false, false));
        assert!(should_multiplex(&c, true, false));
    }

    #[test]
    fn a_dry_run_multiplexes_only_when_the_gate_will_run() {
        let mut c = config(&[]);
        c.policy.tripwire = true;
        assert!(should_multiplex(&c, false, true));
        // `--force` bypasses the gate, so a dry run then does no remote I/O.
        assert!(!should_multiplex(&c, true, true));
        c.policy.tripwire = false;
        assert!(!should_multiplex(&c, false, true));
    }

    #[test]
    fn local_and_container_targets_are_not_multiplexed() {
        let c = config(&[("local", "127.0.0.1"), ("remote", "192.0.2.10")]);
        let picked = ssh_machines_in_scope(&c, None);
        assert_eq!(picked.len(), 1, "{picked:?}");
        assert_eq!(picked[0].addr, "192.0.2.10");
    }

    #[test]
    fn machine_filter_narrows_the_fleet() {
        let c = config(&[("a", "192.0.2.10"), ("b", "192.0.2.11")]);
        assert_eq!(ssh_machines_in_scope(&c, None).len(), 2);
        let only_b = ssh_machines_in_scope(&c, Some("b"));
        assert_eq!(only_b.len(), 1);
        assert_eq!(only_b[0].addr, "192.0.2.11");
        assert!(ssh_machines_in_scope(&c, Some("nope")).is_empty());
    }

    /// No SSH target means no guard, so a purely local apply spawns no ssh.
    #[test]
    fn an_all_local_fleet_opens_no_guard() {
        let c = config(&[("local", "127.0.0.1")]);
        assert!(open_control_masters(&c, None, false, false, false).is_none());
    }
}
