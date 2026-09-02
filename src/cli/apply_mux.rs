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
    // The SAME predicate the gate runs on, not a copy of it: the decision to
    // open a socket for the gate and the decision to run the gate must never
    // drift apart (forjar#404, agy lane).
    super::apply_drift::gate_will_run(config, force)
}

/// The SSH-transport machines this run will actually reach.
///
/// Scoped by every selector the executor honours — `-m`, `-r`, `-t`, `-g`,
/// and the `--exclude`/`--skip`/`--subset` pruning that already happened to
/// `config.resources` — not by `-m` alone. Measured on the first cut: with
/// `-r one-resource` this opened a master to EVERY SSH machine in the file,
/// an O(fleet) setup bill for an O(1) apply. Local, container and pepita
/// targets are excluded by `is_ssh_transport` — there is no handshake to
/// amortise for them.
pub(super) fn ssh_machines_in_scope(
    config: &types::ForjarConfig,
    scope: &super::apply_drift::GateScope<'_>,
) -> Vec<types::Machine> {
    scope
        .machines_in_scope(config)
        .into_iter()
        .filter_map(|name| config.machines.get(&name))
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
    scope: &super::apply_drift::GateScope<'_>,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> Option<transport::ssh_mux::ControlMasterGuard> {
    if !should_multiplex(config, force, dry_run) {
        return None;
    }
    let machines = ssh_machines_in_scope(config, scope);
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

    /// One `file` resource per machine, named `<machine>-f`, tagged with the
    /// machine's name so `-t` can select it.
    fn config(machines: &[(&str, &str)]) -> types::ForjarConfig {
        let mut m = IndexMap::new();
        let mut r = IndexMap::new();
        for (name, addr) in machines {
            m.insert((*name).to_string(), machine(addr));
            r.insert(
                format!("{name}-f"),
                types::Resource {
                    machine: types::MachineTarget::Single((*name).to_string()),
                    tags: vec![(*name).to_string()],
                    ..Default::default()
                },
            );
        }
        types::ForjarConfig {
            machines: m,
            resources: r,
            ..Default::default()
        }
    }

    fn scope<'a>(
        machine: Option<&'a str>,
        resource: Option<&'a str>,
        tag: Option<&'a str>,
    ) -> super::super::apply_drift::GateScope<'a> {
        super::super::apply_drift::GateScope {
            machine,
            resource,
            tag,
            group: None,
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
        let picked = ssh_machines_in_scope(&c, &scope(None, None, None));
        assert_eq!(picked.len(), 1, "{picked:?}");
        assert_eq!(picked[0].addr, "192.0.2.10");
    }

    #[test]
    fn machine_filter_narrows_the_fleet() {
        let c = config(&[("a", "192.0.2.10"), ("b", "192.0.2.11")]);
        assert_eq!(ssh_machines_in_scope(&c, &scope(None, None, None)).len(), 2);
        let only_b = ssh_machines_in_scope(&c, &scope(Some("b"), None, None));
        assert_eq!(only_b.len(), 1);
        assert_eq!(only_b[0].addr, "192.0.2.11");
        assert!(ssh_machines_in_scope(&c, &scope(Some("nope"), None, None)).is_empty());
    }

    /// forjar#404 (agy lane): `-r` and `-t` must narrow the MACHINES too, or a
    /// one-resource apply opens a master to the whole fleet.
    #[test]
    fn resource_and_tag_filters_narrow_the_fleet() {
        let c = config(&[
            ("a", "192.0.2.10"),
            ("b", "192.0.2.11"),
            ("c", "192.0.2.12"),
        ]);
        let only_b = ssh_machines_in_scope(&c, &scope(None, Some("b-f"), None));
        assert_eq!(only_b.len(), 1, "{only_b:?}");
        assert_eq!(only_b[0].addr, "192.0.2.11");
        let only_c = ssh_machines_in_scope(&c, &scope(None, None, Some("c")));
        assert_eq!(only_c.len(), 1, "{only_c:?}");
        assert_eq!(only_c[0].addr, "192.0.2.12");
        assert!(ssh_machines_in_scope(&c, &scope(None, Some("nope"), None)).is_empty());
    }

    /// A machine with no declared resource left — every declaration pruned by
    /// `--exclude`, or none written — gets no socket: nothing will use it.
    #[test]
    fn a_machine_with_nothing_in_scope_gets_no_master() {
        let mut c = config(&[("a", "192.0.2.10"), ("b", "192.0.2.11")]);
        c.resources.shift_remove("b-f");
        let picked = ssh_machines_in_scope(&c, &scope(None, None, None));
        assert_eq!(picked.len(), 1, "{picked:?}");
        assert_eq!(picked[0].addr, "192.0.2.10");
    }

    /// No SSH target means no guard, so a purely local apply spawns no ssh.
    #[test]
    fn an_all_local_fleet_opens_no_guard() {
        let c = config(&[("local", "127.0.0.1")]);
        assert!(open_control_masters(&c, &scope(None, None, None), false, false, false).is_none());
    }
}
