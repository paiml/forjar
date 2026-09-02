//! forjar#404 (CRUX audit E02): an RAII owner for a whole run's ControlMasters.
//!
//! WHY THIS EXISTS. `start_control_master` / `stop_control_master` are a pair
//! that has to be balanced across a scope, and the only caller that balanced
//! them was `executor::machine::apply_machine` — which runs AFTER the pre-apply
//! drift gate. Every gate query therefore found no socket in `build_ssh_args`
//! and paid a full SSH handshake: measured 306 ms median fresh against 6.7 ms
//! multiplexed, a 45× penalty repeated once per locked resource per machine.
//!
//! Hoisting the pair by hand at the top of `cmd_apply` would have been a
//! `Vec<Machine>` and two loops separated by every early `return` in the apply
//! preflight — the gates all return `Err` and there are eleven of them. A guard
//! makes the teardown unmissable.
//!
//! IT ONLY STOPS WHAT IT STARTED. `start_control_master` returns `Ok(false)`
//! when a live master already exists, and a master someone else owns (a
//! concurrent apply, a `ControlPersist` socket from a previous run still inside
//! its 60 s window) must outlive this guard. Adopting it would have this run's
//! `Drop` tear down a connection another process is mid-query on.

use crate::core::types::Machine;

/// Holds SSH ControlMaster sockets open for the machines a run will talk to.
///
/// Sockets opened by this guard are closed when it drops, including on the
/// error paths of every gate between here and the executor.
pub struct ControlMasterGuard {
    /// Only the machines whose master THIS guard opened.
    started: Vec<Machine>,
}

impl ControlMasterGuard {
    /// Open a ControlMaster for each machine, skipping ones already served.
    ///
    /// Opens are fanned out with `std::thread::scope`: a `ConnectTimeout=5`
    /// handshake per machine, taken sequentially, is exactly the serialisation
    /// this change exists to remove — 100 machines would spend half a minute
    /// here before the first query.
    ///
    /// Failure to open is not fatal and not fatal-shaped: the run simply pays
    /// the un-multiplexed price it paid before, and the real connection error
    /// surfaces from the query that needed it, with that query's context.
    pub fn open(machines: &[Machine], verbose: bool) -> Self {
        let opened: Vec<bool> = std::thread::scope(|s| {
            let handles: Vec<_> = machines
                .iter()
                .map(|m| s.spawn(move || super::ssh::start_control_master(m)))
                .collect();
            handles
                .into_iter()
                .zip(machines)
                .map(|(h, m)| match h.join() {
                    Ok(Ok(started)) => started,
                    Ok(Err(e)) => {
                        if verbose {
                            eprintln!("warning: SSH multiplexing failed for {}: {e}", m.hostname);
                        }
                        false
                    }
                    Err(_) => false,
                })
                .collect()
        });

        let started = machines
            .iter()
            .zip(opened)
            .filter(|(_, opened)| *opened)
            .map(|(m, _)| m.clone())
            .collect();
        Self { started }
    }

    /// How many masters this guard actually opened (for tests and tracing).
    pub fn opened(&self) -> usize {
        self.started.len()
    }
}

impl Drop for ControlMasterGuard {
    fn drop(&mut self) {
        for m in &self.started {
            let _ = super::ssh::stop_control_master(m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(addr: &str) -> Machine {
        Machine {
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

    /// An empty fleet must not spawn anything, and must drop cleanly.
    #[test]
    fn empty_guard_opens_nothing() {
        let g = ControlMasterGuard::open(&[], false);
        assert_eq!(g.opened(), 0);
    }

    /// A machine that cannot be reached leaves the guard owning nothing, so
    /// `Drop` cannot tear down a socket it never created.
    #[test]
    fn unreachable_machine_is_not_adopted() {
        // `.invalid` is reserved by RFC 2606 and never resolves, so this fails
        // at name resolution rather than burning `ConnectTimeout` seconds.
        let g = ControlMasterGuard::open(&[machine("forjar-e02.invalid")], false);
        assert_eq!(g.opened(), 0);
    }
}
