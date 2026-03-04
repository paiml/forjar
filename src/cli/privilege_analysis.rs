//! FJ-1403: Least-privilege execution analysis.
//!
//! Analyzes a config and reports the minimum permissions required
//! to converge each resource. Identifies which resources need root/sudo
//! and which can run unprivileged.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// Permission level for a resource.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PrivilegeLevel {
    /// No special permissions needed
    Unprivileged,
    /// Needs file write to system paths
    SystemWrite,
    /// Needs package manager (typically root)
    PackageManager,
    /// Needs service control (typically root)
    ServiceControl,
    /// Needs network configuration (root)
    NetworkConfig,
    /// Explicitly requires sudo
    ExplicitSudo,
}

impl PrivilegeLevel {
    fn label(&self) -> &'static str {
        match self {
            Self::Unprivileged => "unprivileged",
            Self::SystemWrite => "system-write",
            Self::PackageManager => "package-manager",
            Self::ServiceControl => "service-control",
            Self::NetworkConfig => "network-config",
            Self::ExplicitSudo => "sudo",
        }
    }

    fn needs_root(&self) -> bool {
        !matches!(self, Self::Unprivileged)
    }
}

/// Analyze a single resource for minimum privilege level.
fn analyze_resource(resource: &types::Resource) -> PrivilegeLevel {
    if resource.sudo {
        return PrivilegeLevel::ExplicitSudo;
    }

    match resource.resource_type {
        types::ResourceType::Package => PrivilegeLevel::PackageManager,
        types::ResourceType::Service => PrivilegeLevel::ServiceControl,
        types::ResourceType::Network => PrivilegeLevel::NetworkConfig,
        types::ResourceType::Mount => PrivilegeLevel::SystemWrite,
        types::ResourceType::User => PrivilegeLevel::ExplicitSudo,
        types::ResourceType::File => analyze_file_privilege(resource),
        types::ResourceType::Docker => PrivilegeLevel::SystemWrite,
        _ => PrivilegeLevel::Unprivileged,
    }
}

/// Analyze file resource paths to determine if system paths are involved.
fn analyze_file_privilege(resource: &types::Resource) -> PrivilegeLevel {
    let system_prefixes = [
        "/etc/", "/usr/", "/var/", "/opt/", "/root/", "/sys/", "/proc/",
    ];
    if let Some(ref path) = resource.path {
        for prefix in &system_prefixes {
            if path.starts_with(prefix) {
                return PrivilegeLevel::SystemWrite;
            }
        }
    }
    PrivilegeLevel::Unprivileged
}

pub(crate) fn cmd_privilege_analysis(
    file: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let mut entries: Vec<(String, PrivilegeLevel, String)> = Vec::new();

    for (id, resource) in &config.resources {
        if let Some(mf) = machine_filter {
            let machines = resource.machine.to_vec();
            if !machines.iter().any(|m| m == mf) {
                continue;
            }
        }
        let level = analyze_resource(resource);
        let rtype = format!("{:?}", resource.resource_type).to_lowercase();
        entries.push((id.clone(), level, rtype));
    }

    entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    if json {
        print_privilege_json(&entries);
    } else {
        print_privilege_text(&entries);
    }

    Ok(())
}

fn print_privilege_json(entries: &[(String, PrivilegeLevel, String)]) {
    let items: Vec<String> = entries
        .iter()
        .map(|(id, level, rtype)| {
            format!(
                r#"{{"resource":"{}","type":"{}","privilege":"{}","needs_root":{}}}"#,
                id,
                rtype,
                level.label(),
                level.needs_root()
            )
        })
        .collect();

    let root_count = entries.iter().filter(|(_, l, _)| l.needs_root()).count();
    let unpriv_count = entries.len() - root_count;

    println!(
        r#"{{"resources":[{}],"summary":{{"total":{},"needs_root":{},"unprivileged":{}}}}}"#,
        items.join(","),
        entries.len(),
        root_count,
        unpriv_count
    );
}

fn print_privilege_text(entries: &[(String, PrivilegeLevel, String)]) {
    println!("{}\n", bold("Privilege Analysis"));

    let root_count = entries.iter().filter(|(_, l, _)| l.needs_root()).count();
    let unpriv_count = entries.len() - root_count;

    if entries.is_empty() {
        println!("  (no resources)");
        return;
    }

    // Resources needing elevated privileges
    if root_count > 0 {
        println!(
            "  {} Requires elevated privileges:",
            red(&format!("{root_count}"))
        );
        for (id, level, rtype) in entries {
            if level.needs_root() {
                println!(
                    "    {} {} ({}) — {}",
                    yellow("!"),
                    bold(id),
                    dim(rtype),
                    level.label()
                );
            }
        }
        println!();
    }

    // Unprivileged resources
    if unpriv_count > 0 {
        println!(
            "  {} Can run unprivileged:",
            green(&format!("{unpriv_count}"))
        );
        for (id, _level, rtype) in entries {
            if !_level.needs_root() {
                println!("    {} {} ({})", green("*"), id, dim(rtype));
            }
        }
        println!();
    }

    // Summary
    println!(
        "  {} {}/{} resources need root",
        bold("Summary:"),
        root_count,
        entries.len()
    );
    if root_count == 0 {
        println!("  {} This config can run entirely unprivileged", green("✓"));
    }
}
