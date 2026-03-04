//! FJ-1423: Brownfield state import.
//!
//! `forjar import --brownfield` scans a target machine for existing resources
//! (packages, files, services) and generates a forjar config + initial state
//! lock from discovered infrastructure.

use crate::core::types::ResourceType;
use std::path::Path;

/// A discovered resource from brownfield scanning.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscoveredResource {
    pub id: String,
    pub resource_type: String,
    pub properties: std::collections::BTreeMap<String, String>,
    pub source: String,
}

/// Brownfield import report.
#[derive(Debug, serde::Serialize)]
pub struct BrownfieldReport {
    pub machine: String,
    pub resources: Vec<DiscoveredResource>,
    pub total: usize,
    pub generated_config: String,
}

/// Import existing infrastructure as forjar-managed resources.
pub fn cmd_import_brownfield(
    machine: &str,
    scan_types: &[String],
    output: Option<&Path>,
    json: bool,
) -> Result<(), String> {
    let types: Vec<ResourceType> = if scan_types.is_empty() {
        vec![
            ResourceType::Package,
            ResourceType::File,
            ResourceType::Service,
        ]
    } else {
        scan_types
            .iter()
            .filter_map(|t| parse_resource_type(t))
            .collect()
    };

    let mut resources = Vec::new();

    for rt in &types {
        match rt {
            ResourceType::Package => discover_packages(&mut resources, machine),
            ResourceType::Service => discover_services(&mut resources, machine),
            ResourceType::File => discover_config_files(&mut resources, machine),
            _ => {}
        }
    }

    let total = resources.len();
    let config_yaml = generate_config(machine, &resources);

    if let Some(out) = output {
        std::fs::write(out, &config_yaml).map_err(|e| format!("write config: {e}"))?;
    }

    let report = BrownfieldReport {
        machine: machine.to_string(),
        resources,
        total,
        generated_config: config_yaml,
    };

    if json {
        let out =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        println!("Brownfield Import: {machine}");
        println!("Discovered: {total} resources");
        for r in &report.resources {
            println!("  [{:>8}] {}", r.resource_type, r.id);
        }
        if output.is_some() {
            println!("Config written to output file");
        }
    }

    Ok(())
}

pub(crate) fn parse_resource_type(s: &str) -> Option<ResourceType> {
    match s.to_lowercase().as_str() {
        "package" | "pkg" => Some(ResourceType::Package),
        "file" => Some(ResourceType::File),
        "service" | "svc" => Some(ResourceType::Service),
        "docker" => Some(ResourceType::Docker),
        "cron" => Some(ResourceType::Cron),
        _ => None,
    }
}

fn discover_packages(resources: &mut Vec<DiscoveredResource>, _machine: &str) {
    let dpkg_path = Path::new("/var/lib/dpkg/status");
    let Ok(content) = std::fs::read_to_string(dpkg_path) else {
        return;
    };
    for pkg in parse_dpkg_packages(&content).into_iter().take(50) {
        let mut props = std::collections::BTreeMap::new();
        props.insert("provider".to_string(), "apt".to_string());
        resources.push(DiscoveredResource {
            id: format!("pkg-{pkg}"),
            resource_type: "package".to_string(),
            properties: props,
            source: "dpkg".to_string(),
        });
    }
}

pub(crate) fn parse_dpkg_packages(content: &str) -> Vec<String> {
    let mut packages = Vec::new();
    for line in content.lines() {
        if let Some(pkg) = line.strip_prefix("Package: ") {
            packages.push(pkg.trim().to_string());
        }
    }
    packages
}

fn discover_services(resources: &mut Vec<DiscoveredResource>, _machine: &str) {
    let systemd_dir = Path::new("/etc/systemd/system");
    let Ok(entries) = std::fs::read_dir(systemd_dir) else {
        return;
    };
    for entry in entries.flatten().take(50) {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(svc_name) = name.strip_suffix(".service") else {
            continue;
        };
        let mut props = std::collections::BTreeMap::new();
        props.insert("unit".to_string(), name.clone());
        resources.push(DiscoveredResource {
            id: format!("svc-{svc_name}"),
            resource_type: "service".to_string(),
            properties: props,
            source: "systemd".to_string(),
        });
    }
}

fn discover_config_files(resources: &mut Vec<DiscoveredResource>, _machine: &str) {
    let dirs = ["/etc/nginx", "/etc/apache2", "/etc/ssh"];
    for dir in &dirs {
        discover_dir_files(resources, Path::new(dir));
    }
}

fn discover_dir_files(resources: &mut Vec<DiscoveredResource>, dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten().take(20) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let mut props = std::collections::BTreeMap::new();
        props.insert("path".to_string(), path.display().to_string());
        if let Ok(meta) = path.metadata() {
            props.insert("size".to_string(), meta.len().to_string());
        }
        resources.push(DiscoveredResource {
            id: format!("file-{}", name.replace('.', "-")),
            resource_type: "file".to_string(),
            properties: props,
            source: "filesystem".to_string(),
        });
    }
}

pub(crate) fn generate_config(machine: &str, resources: &[DiscoveredResource]) -> String {
    let mut yaml = format!(
        "# Auto-generated by forjar import --brownfield\n\
         version: \"1.0\"\n\
         name: imported-{machine}\n\n\
         machines:\n\
         \x20 {machine}:\n\
         \x20   hostname: {machine}\n\
         \x20   addr: 127.0.0.1\n\n\
         resources:\n"
    );
    for r in resources {
        emit_resource_yaml(&mut yaml, machine, r);
    }
    yaml
}

fn emit_resource_yaml(yaml: &mut String, machine: &str, r: &DiscoveredResource) {
    yaml.push_str(&format!("  {}:\n", r.id));
    yaml.push_str(&format!("    type: {}\n", r.resource_type));
    yaml.push_str(&format!("    machine: {machine}\n"));
    if r.resource_type == "package" {
        let pkg = r.id.strip_prefix("pkg-").unwrap_or(&r.id);
        yaml.push_str(&format!("    packages: [{pkg}]\n"));
    }
    if let Some(path) = r.properties.get("path") {
        yaml.push_str(&format!("    path: {path}\n"));
    }
    yaml.push_str(&format!("    tags: [imported, {}]\n", r.source));
}

