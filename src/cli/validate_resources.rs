//! Resource validation.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// FJ-571: Validate resource counts don't exceed per-machine limits.
pub(crate) fn cmd_validate_check_resource_limits(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let max_resources_per_machine = 100;

    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (_name, res) in &config.resources {
        let machine_name = match &res.machine {
            crate::core::types::MachineTarget::Single(s) => s.clone(),
            crate::core::types::MachineTarget::Multiple(ms) => {
                for m in ms {
                    *counts.entry(m.clone()).or_default() += 1;
                }
                continue;
            }
        };
        *counts.entry(machine_name).or_default() += 1;
    }

    let mut violations: Vec<(String, usize)> = Vec::new();
    for (machine, count) in &counts {
        if *count > max_resources_per_machine {
            violations.push((machine.clone(), *count));
        }
    }

    if json {
        print_resource_limits_json(&counts, max_resources_per_machine, violations.len());
    } else {
        print_resource_limits_text(&counts, &violations, max_resources_per_machine);
    }
    Ok(())
}

/// Print resource limits as JSON.
fn print_resource_limits_json(
    counts: &std::collections::HashMap<String, usize>,
    limit: usize,
    violation_count: usize,
) {
    let items: Vec<String> = counts
        .iter()
        .map(|(m, c)| {
            format!(
                r#"{{"machine":"{}","resources":{},"over_limit":{}}}"#,
                m,
                c,
                c > &limit
            )
        })
        .collect();
    println!(
        r#"{{"resource_limits":[{}],"limit":{},"violations":{}}}"#,
        items.join(","),
        limit,
        violation_count
    );
}

/// Print resource limits as text.
pub(crate) fn print_resource_limits_text(
    counts: &std::collections::HashMap<String, usize>,
    violations: &[(String, usize)],
    limit: usize,
) {
    if violations.is_empty() {
        println!(
            "Resource limits check passed (limit: {} per machine)",
            limit
        );
        for (machine, count) in counts {
            println!("  {} — {} resources", machine, count);
        }
    } else {
        println!("Resource limit violations:");
        for (machine, count) in violations {
            println!("  {} — {} resources (limit: {})", machine, count, limit);
        }
    }
}

/// FJ-581: Detect resources not referenced by any dependency chain.
pub(crate) fn cmd_validate_check_unused(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let mut referenced: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (_name, res) in &config.resources {
        for dep in &res.depends_on {
            referenced.insert(dep.clone());
        }
    }

    let mut unused: Vec<String> = Vec::new();
    for (name, res) in &config.resources {
        if !referenced.contains(name) && res.depends_on.is_empty() && config.resources.len() > 1 {
            unused.push(name.clone());
        }
    }
    unused.sort();

    if json {
        let items: Vec<String> = unused.iter().map(|u| format!(r#""{}""#, u)).collect();
        println!(
            r#"{{"unused":[{}],"count":{}}}"#,
            items.join(","),
            unused.len()
        );
    } else if unused.is_empty() {
        println!("No unused resources found — all resources are part of a dependency chain");
    } else {
        println!("Unused resources ({}):", unused.len());
        for u in &unused {
            println!("  {}", u);
        }
    }
    Ok(())
}

/// FJ-590: Validate dependency graph for unresolved references.
pub(crate) fn cmd_validate_check_dependencies(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut issues: Vec<(String, String)> = Vec::new();

    let resource_names: std::collections::HashSet<&str> =
        config.resources.keys().map(|k| k.as_str()).collect();

    for (rname, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !resource_names.contains(dep.as_str()) {
                issues.push((rname.clone(), dep.clone()));
            }
        }
    }

    if json {
        let items: Vec<String> = issues
            .iter()
            .map(|(r, d)| format!(r#"{{"resource":"{}","missing":"{}"}}"#, r, d))
            .collect();
        println!(
            r#"{{"dependency_issues":[{}],"count":{}}}"#,
            items.join(","),
            issues.len()
        );
    } else if issues.is_empty() {
        println!("All dependency references are valid");
    } else {
        println!("Dependency issues found ({}):", issues.len());
        for (r, d) in &issues {
            println!("  {} -> {} (missing)", r, d);
        }
    }
    Ok(())
}

/// Check permission issues for a single resource.
fn check_resource_permissions(
    rname: &str,
    resource: &types::Resource,
    issues: &mut Vec<(String, String)>,
) {
    if let Some(mode) = &resource.mode {
        if mode.len() == 4 {
            if let Some(last) = mode.chars().last() {
                let val = last.to_digit(8).unwrap_or(0);
                if val & 2 != 0 {
                    issues.push((rname.to_string(), format!("world-writable mode: {}", mode)));
                }
            }
        }
    }
    check_root_on_nonsystem_path(rname, resource, issues);
}

/// Check for root ownership on non-system paths.
fn check_root_on_nonsystem_path(
    rname: &str,
    resource: &types::Resource,
    issues: &mut Vec<(String, String)>,
) {
    if let Some(owner) = &resource.owner {
        if owner == "root" {
            if let Some(path) = &resource.path {
                if !path.starts_with("/etc")
                    && !path.starts_with("/usr")
                    && !path.starts_with("/var")
                {
                    issues.push((
                        rname.to_string(),
                        format!("root ownership on non-system path: {}", path),
                    ));
                }
            }
        }
    }
}

/// FJ-601: Validate resource ownership/mode fields are secure.
pub(crate) fn cmd_validate_check_permissions(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut issues: Vec<(String, String)> = Vec::new();

    for (rname, resource) in &config.resources {
        check_resource_permissions(rname, resource, &mut issues);
    }

    if json {
        let items: Vec<String> = issues
            .iter()
            .map(|(r, msg)| format!(r#"{{"resource":"{}","issue":"{}"}}"#, r, msg))
            .collect();
        println!(
            r#"{{"permission_issues":[{}],"count":{}}}"#,
            items.join(","),
            issues.len()
        );
    } else if issues.is_empty() {
        println!("All resource permissions look secure");
    } else {
        println!("Permission issues found ({}):", issues.len());
        for (r, msg) in &issues {
            println!("  {} — {}", r, msg);
        }
    }
    Ok(())
}

/// FJ-621: Verify machines are reachable by checking addr format.
pub(crate) fn cmd_validate_check_machine_reachability(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut reachable = 0u64;
    let mut unreachable: Vec<(String, String)> = Vec::new();

    for (mname, machine) in &config.machines {
        let addr = &machine.addr;
        if addr == "127.0.0.1"
            || addr == "localhost"
            || addr == "container"
            || addr.contains('.')
            || addr.contains(':')
        {
            reachable += 1;
        } else {
            unreachable.push((mname.clone(), addr.clone()));
        }
    }

    if json {
        let items: Vec<String> = unreachable
            .iter()
            .map(|(m, a)| format!(r#"{{"machine":"{}","addr":"{}"}}"#, m, a))
            .collect();
        println!(
            r#"{{"reachable":{},"unreachable":[{}],"count":{}}}"#,
            reachable,
            items.join(","),
            unreachable.len()
        );
    } else if unreachable.is_empty() {
        println!("All {} machines appear reachable", reachable);
    } else {
        println!(
            "Machine reachability ({} ok, {} suspect):",
            reachable,
            unreachable.len()
        );
        for (m, a) in &unreachable {
            println!("  {} — invalid addr: {}", m, a);
        }
    }
    Ok(())
}

/// FJ-661: Validate owner consistency across resources
pub(crate) fn cmd_validate_check_owner_consistency(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

    let mut machine_owners: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();
    for (name, resource) in &config.resources {
        let machine = resource.machine.to_string();
        let owner = resource
            .owner
            .clone()
            .unwrap_or_else(|| "unset".to_string());
        machine_owners
            .entry(machine)
            .or_default()
            .push((name.clone(), owner));
    }

    let mut inconsistencies = Vec::new();
    for (machine, resources) in &machine_owners {
        let owners: std::collections::HashSet<&str> =
            resources.iter().map(|(_, o)| o.as_str()).collect();
        if owners.len() > 1 {
            let owner_list: Vec<_> = owners.into_iter().collect();
            inconsistencies.push(format!(
                "Machine '{}': mixed owners [{}]",
                machine,
                owner_list.join(", ")
            ));
        }
    }

    if json {
        print!("{{\"inconsistencies\":[");
        for (i, inc) in inconsistencies.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(r#""{}""#, inc.replace('"', "\\\""));
        }
        println!("]}}");
    } else if inconsistencies.is_empty() {
        println!("All machines have consistent resource ownership");
    } else {
        println!("Owner inconsistencies ({}):", inconsistencies.len());
        for inc in &inconsistencies {
            println!("  - {}", inc);
        }
    }
    Ok(())
}

/// FJ-681: Validate service dependency chains
pub(crate) fn cmd_validate_check_service_deps(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

    let resource_names: std::collections::HashSet<String> =
        config.resources.keys().cloned().collect();
    let mut missing_deps = Vec::new();

    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !resource_names.contains(dep) {
                missing_deps.push(format!(
                    "Resource '{}' depends on '{}' which does not exist",
                    name, dep
                ));
            }
        }
    }

    if json {
        print!("{{\"missing_deps\":[");
        for (i, d) in missing_deps.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(r#""{}""#, d.replace('"', "\\\""));
        }
        println!("]}}");
    } else if missing_deps.is_empty() {
        println!("All service dependency chains are satisfiable");
    } else {
        println!("Missing dependencies ({}):", missing_deps.len());
        for d in &missing_deps {
            println!("  - {}", d);
        }
    }
    Ok(())
}
