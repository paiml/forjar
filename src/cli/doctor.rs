//! Doctor diagnostics.

use super::helpers::*;
use crate::core::{parser, secrets, types};
use std::path::Path;

#[derive(Debug)]
struct DoctorCheck {
    name: String,
    status: DoctorStatus,
    detail: String,
}

#[derive(Debug, PartialEq)]
enum DoctorStatus {
    Pass,
    Warn,
    Fail,
}

impl DoctorStatus {
    fn label(&self) -> &'static str {
        match self {
            DoctorStatus::Pass => "pass",
            DoctorStatus::Warn => "warn",
            DoctorStatus::Fail => "FAIL",
        }
    }

    fn json_label(&self) -> &'static str {
        match self {
            DoctorStatus::Pass => "pass",
            DoctorStatus::Warn => "warn",
            DoctorStatus::Fail => "fail",
        }
    }
}

fn check_bash() -> DoctorCheck {
    use std::process::Command;
    match Command::new("bash").arg("--version").output() {
        Ok(out) => {
            let ver = String::from_utf8_lossy(&out.stdout);
            let version_str = ver.lines().next().unwrap_or("").to_string();
            if let Some(pos) = version_str.find("version ") {
                let after = &version_str[pos + 8..];
                let major: u32 = after
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0);
                if major >= 4 {
                    DoctorCheck {
                        name: "bash".to_string(),
                        status: DoctorStatus::Pass,
                        detail: format!(
                            "bash {}",
                            &after[..after
                                .find(|c: char| c.is_whitespace() || c == '(')
                                .unwrap_or(after.len())]
                        ),
                    }
                } else {
                    DoctorCheck {
                        name: "bash".to_string(),
                        status: DoctorStatus::Fail,
                        detail: format!("bash {major} (need >= 4.0)"),
                    }
                }
            } else {
                DoctorCheck {
                    name: "bash".to_string(),
                    status: DoctorStatus::Warn,
                    detail: "cannot parse bash version".to_string(),
                }
            }
        }
        Err(_) => DoctorCheck {
            name: "bash".to_string(),
            status: DoctorStatus::Fail,
            detail: "bash not found in PATH".to_string(),
        },
    }
}

fn check_ssh() -> DoctorCheck {
    use std::process::Command;
    match Command::new("ssh").arg("-V").output() {
        Ok(out) => {
            let ver = String::from_utf8_lossy(&out.stderr);
            let version_line = ver.lines().next().unwrap_or("ssh available").to_string();
            DoctorCheck {
                name: "ssh".to_string(),
                status: DoctorStatus::Pass,
                detail: version_line,
            }
        }
        Err(_) => DoctorCheck {
            name: "ssh".to_string(),
            status: DoctorStatus::Fail,
            detail: "ssh not found (needed for remote machines)".to_string(),
        },
    }
}

fn check_container_runtime(runtime: &str) -> DoctorCheck {
    use std::process::Command;
    match Command::new(runtime).arg("--version").output() {
        Ok(out) => {
            let ver = String::from_utf8_lossy(&out.stdout);
            let version_line = ver.lines().next().unwrap_or(runtime).trim().to_string();
            DoctorCheck {
                name: runtime.to_string(),
                status: DoctorStatus::Pass,
                detail: version_line,
            }
        }
        Err(_) => DoctorCheck {
            name: runtime.to_string(),
            status: DoctorStatus::Fail,
            detail: format!("{runtime} not found (needed for container machines)"),
        },
    }
}

fn check_age_identity() -> DoctorCheck {
    #[cfg(not(feature = "encryption"))]
    {
        DoctorCheck {
            name: "age".to_string(),
            status: DoctorStatus::Warn,
            detail: "encryption feature not compiled in".to_string(),
        }
    }
    #[cfg(feature = "encryption")]
    match secrets::load_identities(None) {
        Ok(ids) if !ids.is_empty() => DoctorCheck {
            name: "age".to_string(),
            status: DoctorStatus::Pass,
            detail: format!("{} identity loaded", ids.len()),
        },
        Ok(_) => DoctorCheck {
            name: "age".to_string(),
            status: DoctorStatus::Fail,
            detail: "no age identity (set FORJAR_AGE_KEY or use --identity)".to_string(),
        },
        Err(e) => DoctorCheck {
            name: "age".to_string(),
            status: DoctorStatus::Fail,
            detail: format!("age identity error: {e}"),
        },
    }
}

fn check_state_dir_existence(state_dir: &Path, fix: bool) -> DoctorCheck {
    if state_dir.exists() {
        let test_path = state_dir.join(".doctor-probe");
        match std::fs::write(&test_path, "probe") {
            Ok(()) => {
                let _ = std::fs::remove_file(&test_path);
                DoctorCheck {
                    name: "state-dir".to_string(),
                    status: DoctorStatus::Pass,
                    detail: format!("{} writable", state_dir.display()),
                }
            }
            Err(e) => DoctorCheck {
                name: "state-dir".to_string(),
                status: DoctorStatus::Fail,
                detail: format!("{} not writable: {}", state_dir.display(), e),
            },
        }
    } else if fix {
        match std::fs::create_dir_all(state_dir) {
            Ok(()) => DoctorCheck {
                name: "state-dir".to_string(),
                status: DoctorStatus::Pass,
                detail: format!("{} created (--fix)", state_dir.display()),
            },
            Err(e) => DoctorCheck {
                name: "state-dir".to_string(),
                status: DoctorStatus::Fail,
                detail: format!("cannot create {}: {}", state_dir.display(), e),
            },
        }
    } else {
        DoctorCheck {
            name: "state-dir".to_string(),
            status: DoctorStatus::Warn,
            detail: format!(
                "{} does not exist (will be created on apply)",
                state_dir.display()
            ),
        }
    }
}

fn check_stale_lock(state_dir: &Path, fix: bool) -> Option<DoctorCheck> {
    if !state_dir.exists() {
        return None;
    }
    let lock_path = state_dir.join(".forjar.lock");
    if !lock_path.exists() {
        return None;
    }
    if fix {
        match std::fs::remove_file(&lock_path) {
            Ok(()) => Some(DoctorCheck {
                name: "lock".to_string(),
                status: DoctorStatus::Pass,
                detail: "stale lock removed (--fix)".to_string(),
            }),
            Err(e) => Some(DoctorCheck {
                name: "lock".to_string(),
                status: DoctorStatus::Fail,
                detail: format!("cannot remove lock: {e}"),
            }),
        }
    } else {
        Some(DoctorCheck {
            name: "lock".to_string(),
            status: DoctorStatus::Warn,
            detail: "stale lock file exists (use --fix to remove)".to_string(),
        })
    }
}

fn check_state_dir(fix: bool) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    let state_dir = Path::new("state");
    checks.push(check_state_dir_existence(state_dir, fix));
    if let Some(lock_check) = check_stale_lock(state_dir, fix) {
        checks.push(lock_check);
    }
    checks
}

fn check_git() -> DoctorCheck {
    use std::process::Command;
    match Command::new("git").args(["status", "--porcelain"]).output() {
        Ok(out) if out.status.success() => {
            let output = String::from_utf8_lossy(&out.stdout);
            if output.trim().is_empty() {
                DoctorCheck {
                    name: "git".to_string(),
                    status: DoctorStatus::Pass,
                    detail: "working tree clean".to_string(),
                }
            } else {
                let line_count = output.lines().count();
                DoctorCheck {
                    name: "git".to_string(),
                    status: DoctorStatus::Warn,
                    detail: format!("{line_count} uncommitted changes"),
                }
            }
        }
        Ok(_) => DoctorCheck {
            name: "git".to_string(),
            status: DoctorStatus::Warn,
            detail: "not a git repository".to_string(),
        },
        Err(_) => DoctorCheck {
            name: "git".to_string(),
            status: DoctorStatus::Warn,
            detail: "git not found in PATH".to_string(),
        },
    }
}

fn output_doctor_checks(checks: &[DoctorCheck], json: bool) {
    if json {
        println!("[");
        for (i, c) in checks.iter().enumerate() {
            let comma = if i + 1 < checks.len() { "," } else { "" };
            println!(
                "  {{\"name\":\"{}\",\"status\":\"{}\",\"detail\":\"{}\"}}{}",
                c.name,
                c.status.json_label(),
                c.detail.replace('\"', "\\\""),
                comma
            );
        }
        println!("]");
    } else {
        for c in checks {
            println!("[{:>4}] {}: {}", c.status.label(), c.name, c.detail);
        }
        let pass_count = checks
            .iter()
            .filter(|c| c.status == DoctorStatus::Pass)
            .count();
        let warn_count = checks
            .iter()
            .filter(|c| c.status == DoctorStatus::Warn)
            .count();
        let fail_count = checks
            .iter()
            .filter(|c| c.status == DoctorStatus::Fail)
            .count();
        println!(
            "\n{} checks: {} pass, {} warn, {} fail",
            checks.len(),
            pass_count,
            warn_count,
            fail_count
        );
    }
}

/// FJ-2603: Check sandbox backend availability for `forjar test`.
fn check_sandbox_backends() -> DoctorCheck {
    use crate::core::store::convergence_runner::backend_available;
    use crate::core::types::SandboxBackend;

    let pepita = backend_available(SandboxBackend::Pepita);
    let container = backend_available(SandboxBackend::Container);
    let chroot = backend_available(SandboxBackend::Chroot);

    let mut available = Vec::new();
    if pepita {
        available.push("pepita");
    }
    if container {
        available.push("container");
    }
    if chroot {
        available.push("chroot");
    }

    if available.is_empty() {
        DoctorCheck {
            name: "sandbox".to_string(),
            status: DoctorStatus::Warn,
            detail: "no sandbox backends available (forjar test runs in simulated mode)"
                .to_string(),
        }
    } else {
        DoctorCheck {
            name: "sandbox".to_string(),
            status: DoctorStatus::Pass,
            detail: format!("backends: {}", available.join(", ")),
        }
    }
}

// FJ-251: forjar doctor — pre-flight system checker
pub(crate) fn cmd_doctor(file: Option<&Path>, json: bool, fix: bool) -> Result<(), String> {
    let mut checks: Vec<DoctorCheck> = Vec::new();

    checks.push(check_bash());

    let config: Option<types::ForjarConfig> = if let Some(f) = file {
        match parser::parse_and_validate(f) {
            Ok(c) => Some(c),
            Err(e) => {
                checks.push(DoctorCheck {
                    name: "config".to_string(),
                    status: DoctorStatus::Fail,
                    detail: format!("parse error: {e}"),
                });
                None
            }
        }
    } else {
        None
    };

    let has_ssh_machines = config
        .as_ref()
        .map(|c| {
            c.machines.values().any(|m| {
                m.transport.as_deref() != Some("container")
                    && m.addr != "127.0.0.1"
                    && m.addr != "localhost"
                    && m.addr != "container"
            })
        })
        .unwrap_or(false);

    let has_container_machines = config
        .as_ref()
        .map(|c| {
            c.machines
                .values()
                .any(|m| m.transport.as_deref() == Some("container") || m.addr == "container")
        })
        .unwrap_or(false);

    let has_enc_markers = file
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|content| secrets::has_encrypted_markers(&content))
        .unwrap_or(false);

    if has_ssh_machines {
        checks.push(check_ssh());
    }

    if has_container_machines {
        let runtime = config
            .as_ref()
            .and_then(|c| {
                c.machines
                    .values()
                    .find_map(|m| m.container.as_ref().map(|ct| ct.runtime.clone()))
            })
            .unwrap_or_else(|| "docker".to_string());
        checks.push(check_container_runtime(&runtime));
    }

    if has_enc_markers {
        checks.push(check_age_identity());
    }

    checks.extend(check_state_dir(fix));
    checks.push(check_git());
    checks.push(check_sandbox_backends());

    output_doctor_checks(&checks, json);

    let has_failures = checks.iter().any(|c| c.status == DoctorStatus::Fail);
    if has_failures {
        Err("doctor found failures".to_string())
    } else {
        Ok(())
    }
}

/// FJ-343: Doctor network check — test SSH to all machines.
pub(crate) fn cmd_doctor_network(file: Option<&Path>, json: bool) -> Result<(), String> {
    let config_path = file.unwrap_or_else(|| std::path::Path::new("forjar.yaml"));
    let config = parse_and_validate(config_path)?;

    let mut results: Vec<serde_json::Value> = Vec::new();

    for (name, machine) in &config.machines {
        let is_local = machine.addr == "127.0.0.1" || machine.addr == "localhost";

        let (status, latency_ms) = if is_local {
            ("reachable".to_string(), 0u64)
        } else {
            let start = std::time::Instant::now();
            let user_host = format!("{}@{}", machine.user, machine.addr);
            let mut ssh_args = vec!["-o", "BatchMode=yes", "-o", "ConnectTimeout=5"];
            if let Some(ref key) = machine.ssh_key {
                ssh_args.push("-i");
                ssh_args.push(key);
            }
            ssh_args.push(&user_host);
            ssh_args.push("echo");
            ssh_args.push("ok");
            let result = std::process::Command::new("ssh")
                .args(&ssh_args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            let elapsed = start.elapsed().as_millis() as u64;
            match result {
                Ok(s) if s.success() => ("reachable".to_string(), elapsed),
                _ => ("unreachable".to_string(), elapsed),
            }
        };

        if json {
            results.push(serde_json::json!({
                "machine": name,
                "addr": machine.addr,
                "status": status,
                "latency_ms": latency_ms,
            }));
        } else {
            let icon = if status == "reachable" {
                green("●")
            } else {
                red("✗")
            };
            println!(
                "  {} {} ({}) — {} ({}ms)",
                icon, name, machine.addr, status, latency_ms
            );
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
    }

    Ok(())
}
