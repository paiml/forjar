//! FJ-2300/E19: Active machine connectivity probing.
//!
//! Probes each machine's transport to verify reachability.
//! SSH machines: `ssh -o ConnectTimeout=5 user@addr true`
//! Container machines: `docker exec <name> true`
//! Local machines: always reachable.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// Connectivity probe result for a single machine.
#[derive(Debug, serde::Serialize)]
struct ConnectivityResult {
    machine: String,
    transport: String,
    reachable: bool,
    latency_ms: Option<u64>,
    error: Option<String>,
}

/// Probe a single machine's connectivity.
fn probe_machine(name: &str, machine: &types::Machine) -> ConnectivityResult {
    let transport = machine
        .transport
        .as_ref()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "local".into());

    let start = std::time::Instant::now();

    match transport.as_str() {
        "local" => ConnectivityResult {
            machine: name.into(),
            transport,
            reachable: true,
            latency_ms: Some(0),
            error: None,
        },
        "ssh" => probe_ssh(name, machine, &transport),
        "container" => probe_container(name, machine, &transport),
        _ => ConnectivityResult {
            machine: name.into(),
            transport,
            reachable: false,
            latency_ms: None,
            error: Some("unknown transport".into()),
        },
    }
    .with_latency(start)
}

impl ConnectivityResult {
    fn with_latency(mut self, start: std::time::Instant) -> Self {
        if self.reachable && self.latency_ms.is_none() {
            self.latency_ms = Some(start.elapsed().as_millis() as u64);
        }
        self
    }
}

fn probe_ssh(name: &str, machine: &types::Machine, transport: &str) -> ConnectivityResult {
    let addr = &machine.addr;
    let user = if machine.user.is_empty() {
        "root"
    } else {
        &machine.user
    };
    let output = std::process::Command::new("ssh")
        .args([
            "-o",
            "ConnectTimeout=5",
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            &format!("{user}@{addr}"),
            "true",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => ConnectivityResult {
            machine: name.into(),
            transport: transport.into(),
            reachable: true,
            latency_ms: None,
            error: None,
        },
        Ok(out) => ConnectivityResult {
            machine: name.into(),
            transport: transport.into(),
            reachable: false,
            latency_ms: None,
            error: Some(String::from_utf8_lossy(&out.stderr).trim().to_string()),
        },
        Err(e) => ConnectivityResult {
            machine: name.into(),
            transport: transport.into(),
            reachable: false,
            latency_ms: None,
            error: Some(e.to_string()),
        },
    }
}

fn probe_container(name: &str, machine: &types::Machine, transport: &str) -> ConnectivityResult {
    let container_name = machine
        .container
        .as_ref()
        .and_then(|c| c.name.as_deref())
        .unwrap_or(name);

    let runtime = machine
        .container
        .as_ref()
        .map(|c| c.runtime.as_str())
        .unwrap_or("docker");

    let output = std::process::Command::new(runtime)
        .args(["exec", container_name, "true"])
        .output();

    match output {
        Ok(out) if out.status.success() => ConnectivityResult {
            machine: name.into(),
            transport: transport.into(),
            reachable: true,
            latency_ms: None,
            error: None,
        },
        Ok(out) => ConnectivityResult {
            machine: name.into(),
            transport: transport.into(),
            reachable: false,
            latency_ms: None,
            error: Some(String::from_utf8_lossy(&out.stderr).trim().to_string()),
        },
        Err(e) => ConnectivityResult {
            machine: name.into(),
            transport: transport.into(),
            reachable: false,
            latency_ms: None,
            error: Some(e.to_string()),
        },
    }
}

/// FJ-2300/E19: Probe all machines for connectivity.
pub(crate) fn cmd_status_connectivity(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("read config: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("parse config: {e}"))?;

    let results: Vec<ConnectivityResult> = config
        .machines
        .iter()
        .map(|(name, machine)| probe_machine(name, machine))
        .collect();

    let reachable = results.iter().filter(|r| r.reachable).count();
    let total = results.len();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
    } else {
        for r in &results {
            let status = if r.reachable {
                green("reachable")
            } else {
                red("unreachable")
            };
            let latency = r
                .latency_ms
                .map(|ms| format!(" ({ms}ms)"))
                .unwrap_or_default();
            let err = r
                .error
                .as_ref()
                .map(|e| format!(" — {e}"))
                .unwrap_or_default();
            println!(
                "  {} {} [{}] {status}{latency}{err}",
                if r.reachable {
                    green("●")
                } else {
                    red("●")
                },
                r.machine,
                r.transport,
            );
        }
        println!("\nConnectivity: {reachable}/{total} machines reachable");
    }

    Ok(())
}
