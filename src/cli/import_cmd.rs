//! Import infrastructure.

use crate::core::types;
use crate::transport;
use std::path::Path;

fn scan_packages(
    machine: &types::Machine,
    machine_name: &str,
    verbose: bool,
    smart: bool,
) -> (String, usize) {
    let mut yaml = String::new();
    let mut count = 0;
    if verbose {
        eprintln!("Scanning installed packages on {}...", machine.addr);
    }

    // Smart mode: only manually installed packages (not auto-installed deps)
    let script = if smart {
        "apt-mark showmanual 2>/dev/null | sort"
    } else {
        "dpkg-query -W -f='${Package}\\n' 2>/dev/null | sort | head -100"
    };

    // Smart mode: get base system packages to exclude
    let base_packages: std::collections::HashSet<String> = if smart {
        get_base_manifest_packages(machine, verbose)
    } else {
        std::collections::HashSet::new()
    };

    match transport::exec_script(machine, script) {
        Ok(output) => {
            let packages: Vec<&str> = output
                .stdout
                .lines()
                .filter(|l| !l.is_empty())
                .filter(|l| !smart || !base_packages.contains(*l))
                .take(if smart { 200 } else { 50 })
                .collect();
            if !packages.is_empty() {
                yaml.push_str("  imported-packages:\n");
                yaml.push_str("    type: package\n");
                yaml.push_str(&format!("    machine: {machine_name}\n"));
                yaml.push_str("    provider: apt\n");
                yaml.push_str("    packages:\n");
                for pkg in &packages {
                    yaml.push_str(&format!("      - {pkg}\n"));
                }
                yaml.push('\n');
                count += 1;
                if verbose {
                    eprintln!("  Found {} packages", packages.len());
                }
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("  Package scan failed: {e}");
            }
        }
    }
    (yaml, count)
}

fn scan_services(machine: &types::Machine, machine_name: &str, verbose: bool) -> (String, usize) {
    let mut yaml = String::new();
    let mut count = 0;
    if verbose {
        eprintln!("Scanning enabled services on {}...", machine.addr);
    }
    let script =
        "systemctl list-unit-files --type=service --state=enabled --no-legend 2>/dev/null \
         | awk '{print $1}' | sed 's/\\.service$//' | sort";
    match transport::exec_script(machine, script) {
        Ok(output) => {
            let services: Vec<&str> = output
                .stdout
                .lines()
                .filter(|l| !l.is_empty() && !l.starts_with("UNIT"))
                .collect();
            for svc in &services {
                let id = format!("svc-{}", svc.replace('.', "-"));
                yaml.push_str(&format!("  {id}:\n"));
                yaml.push_str("    type: service\n");
                yaml.push_str(&format!("    machine: {machine_name}\n"));
                yaml.push_str(&format!("    name: {svc}\n"));
                yaml.push_str("    state: running\n");
                yaml.push_str("    enabled: true\n\n");
                count += 1;
            }
            if verbose {
                eprintln!("  Found {} enabled services", services.len());
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("  Service scan failed: {e}");
            }
        }
    }
    (yaml, count)
}

fn scan_files(machine: &types::Machine, machine_name: &str, verbose: bool) -> (String, usize) {
    let mut yaml = String::new();
    let mut count = 0;
    if verbose {
        eprintln!("Scanning config files on {}...", machine.addr);
    }
    let script = "find /etc -maxdepth 2 -name '*.conf' -type f 2>/dev/null | sort | head -20";
    match transport::exec_script(machine, script) {
        Ok(output) => {
            let files: Vec<&str> = output.stdout.lines().filter(|l| !l.is_empty()).collect();
            for file_path in &files {
                let basename = std::path::Path::new(file_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("config");
                let id = format!("file-{}", basename.replace('.', "-"));
                yaml.push_str(&format!("  {id}:\n"));
                yaml.push_str("    type: file\n");
                yaml.push_str(&format!("    machine: {machine_name}\n"));
                yaml.push_str(&format!("    path: {file_path}\n"));
                yaml.push_str(&format!("    # source: configs{file_path}\n"));
                yaml.push_str("    owner: root\n");
                yaml.push_str("    group: root\n");
                yaml.push_str("    mode: \"0644\"\n\n");
                count += 1;
            }
            if verbose {
                eprintln!("  Found {} config files", files.len());
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("  File scan failed: {e}");
            }
        }
    }
    (yaml, count)
}

fn scan_users(machine: &types::Machine, machine_name: &str, verbose: bool) -> (String, usize) {
    let mut yaml = String::new();
    let mut count = 0;
    if verbose {
        eprintln!("Scanning local users on {}...", machine.addr);
    }
    let script = "awk -F: '$3 >= 1000 && $3 < 65534 {print $1\":\"$6\":\"$7}' /etc/passwd";
    match transport::exec_script(machine, script) {
        Ok(output) => {
            let users: Vec<&str> = output.stdout.lines().filter(|l| !l.is_empty()).collect();
            for user_line in &users {
                let parts: Vec<&str> = user_line.split(':').collect();
                if parts.len() >= 3 {
                    let uname = parts[0];
                    let home = parts[1];
                    let shell = parts[2];
                    let id = format!("user-{uname}");
                    yaml.push_str(&format!("  {id}:\n"));
                    yaml.push_str("    type: user\n");
                    yaml.push_str(&format!("    machine: {machine_name}\n"));
                    yaml.push_str(&format!("    name: {uname}\n"));
                    yaml.push_str(&format!("    home: {home}\n"));
                    yaml.push_str(&format!("    shell: {shell}\n\n"));
                    count += 1;
                }
            }
            if verbose {
                eprintln!("  Found {} users", users.len());
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("  User scan failed: {e}");
            }
        }
    }
    (yaml, count)
}

fn scan_cron(machine: &types::Machine, machine_name: &str, verbose: bool) -> (String, usize) {
    let mut yaml = String::new();
    let mut count = 0;
    if verbose {
        eprintln!("Scanning cron jobs on {}...", machine.addr);
    }
    let script = "crontab -l 2>/dev/null | grep -v '^#' | grep -v '^$' || true";
    match transport::exec_script(machine, script) {
        Ok(output) => {
            let jobs: Vec<&str> = output.stdout.lines().filter(|l| !l.is_empty()).collect();
            for (i, job) in jobs.iter().enumerate() {
                let parts: Vec<&str> = job.splitn(6, ' ').collect();
                if parts.len() >= 6 {
                    let schedule = parts[..5].join(" ");
                    let command = parts[5];
                    let id = format!("cron-job-{}", i + 1);
                    yaml.push_str(&format!("  {id}:\n"));
                    yaml.push_str("    type: cron\n");
                    yaml.push_str(&format!("    machine: {machine_name}\n"));
                    yaml.push_str(&format!("    name: imported-cron-{}\n", i + 1));
                    yaml.push_str(&format!("    schedule: \"{schedule}\"\n"));
                    yaml.push_str(&format!("    command: {command}\n"));
                    yaml.push_str("    owner: root\n\n");
                    count += 1;
                }
            }
            if verbose {
                eprintln!("  Found {} cron jobs", jobs.len());
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("  Cron scan failed: {e}");
            }
        }
    }
    (yaml, count)
}

/// Get packages from the Ubuntu base manifest (base system packages to exclude in smart mode).
fn get_base_manifest_packages(
    machine: &types::Machine,
    verbose: bool,
) -> std::collections::HashSet<String> {
    let script =
        "cat /usr/share/ubuntu-manifest/*.manifest 2>/dev/null | awk '{print $1}' | sort -u";
    match transport::exec_script(machine, script) {
        Ok(output) => {
            let pkgs: std::collections::HashSet<String> = output
                .stdout
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.split(':').next().unwrap_or(l).to_string()) // strip :amd64 suffix
                .collect();
            if verbose {
                eprintln!("  Base manifest: {} packages excluded", pkgs.len());
            }
            pkgs
        }
        Err(_) => {
            if verbose {
                eprintln!("  No base manifest found, using apt-mark only");
            }
            std::collections::HashSet::new()
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_import(
    addr: &str,
    user: &str,
    name: Option<&str>,
    output: &Path,
    scan: &[String],
    verbose: bool,
    smart: bool,
) -> Result<(), String> {
    let machine_name = name.unwrap_or_else(|| {
        if addr == "localhost" || addr == "127.0.0.1" {
            "localhost"
        } else {
            addr.split('.').next().unwrap_or("imported")
        }
    });

    let machine = types::Machine::ssh(machine_name, addr, user);

    let scan_set: std::collections::HashSet<&str> = scan.iter().map(|s| s.as_str()).collect();

    let mut resources_yaml = String::new();
    let mut resource_count = 0;

    if scan_set.contains("packages") {
        let (yaml, count) = scan_packages(&machine, machine_name, verbose, smart);
        resources_yaml.push_str(&yaml);
        resource_count += count;
    }
    if scan_set.contains("services") {
        let (yaml, count) = scan_services(&machine, machine_name, verbose);
        resources_yaml.push_str(&yaml);
        resource_count += count;
    }
    if scan_set.contains("files") {
        let (yaml, count) = scan_files(&machine, machine_name, verbose);
        resources_yaml.push_str(&yaml);
        resource_count += count;
    }
    if scan_set.contains("users") {
        let (yaml, count) = scan_users(&machine, machine_name, verbose);
        resources_yaml.push_str(&yaml);
        resource_count += count;
    }
    if scan_set.contains("cron") {
        let (yaml, count) = scan_cron(&machine, machine_name, verbose);
        resources_yaml.push_str(&yaml);
        resource_count += count;
    }

    let config_yaml = format!(
        r#"# Generated by: forjar import --addr {addr}
# Review and customize before applying.
version: "1.0"
name: imported-{machine_name}

machines:
  {machine_name}:
    hostname: {machine_name}
    addr: {addr}
    user: {user}

resources:
{resources_yaml}
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#,
    );

    std::fs::write(output, &config_yaml)
        .map_err(|e| format!("cannot write {}: {}", output.display(), e))?;

    println!(
        "Imported {} resources from {} → {}",
        resource_count,
        addr,
        output.display()
    );
    Ok(())
}
