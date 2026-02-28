//! Import infrastructure.

use crate::core::types;
use crate::transport;
use std::path::Path;


fn scan_packages(machine: &types::Machine, machine_name: &str, verbose: bool) -> (String, usize) {
    let mut yaml = String::new();
    let mut count = 0;
    if verbose {
        eprintln!("Scanning installed packages on {}...", machine.addr);
    }
    let script = "dpkg-query -W -f='${Package}\\n' 2>/dev/null | sort | head -100";
    match transport::exec_script(machine, script) {
        Ok(output) => {
            let packages: Vec<&str> = output
                .stdout
                .lines()
                .filter(|l| !l.is_empty())
                .take(50)
                .collect();
            if !packages.is_empty() {
                yaml.push_str("  imported-packages:\n");
                yaml.push_str("    type: package\n");
                yaml.push_str(&format!("    machine: {}\n", machine_name));
                yaml.push_str("    provider: apt\n");
                yaml.push_str("    packages:\n");
                for pkg in &packages {
                    yaml.push_str(&format!("      - {}\n", pkg));
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
                eprintln!("  Package scan failed: {}", e);
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
                yaml.push_str(&format!("  {}:\n", id));
                yaml.push_str("    type: service\n");
                yaml.push_str(&format!("    machine: {}\n", machine_name));
                yaml.push_str(&format!("    name: {}\n", svc));
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
                eprintln!("  Service scan failed: {}", e);
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
                yaml.push_str(&format!("  {}:\n", id));
                yaml.push_str("    type: file\n");
                yaml.push_str(&format!("    machine: {}\n", machine_name));
                yaml.push_str(&format!("    path: {}\n", file_path));
                yaml.push_str(&format!("    # source: configs{}\n", file_path));
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
                eprintln!("  File scan failed: {}", e);
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
                    let id = format!("user-{}", uname);
                    yaml.push_str(&format!("  {}:\n", id));
                    yaml.push_str("    type: user\n");
                    yaml.push_str(&format!("    machine: {}\n", machine_name));
                    yaml.push_str(&format!("    name: {}\n", uname));
                    yaml.push_str(&format!("    home: {}\n", home));
                    yaml.push_str(&format!("    shell: {}\n\n", shell));
                    count += 1;
                }
            }
            if verbose {
                eprintln!("  Found {} users", users.len());
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("  User scan failed: {}", e);
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
                    yaml.push_str(&format!("  {}:\n", id));
                    yaml.push_str("    type: cron\n");
                    yaml.push_str(&format!("    machine: {}\n", machine_name));
                    yaml.push_str(&format!("    name: imported-cron-{}\n", i + 1));
                    yaml.push_str(&format!("    schedule: \"{}\"\n", schedule));
                    yaml.push_str(&format!("    command: {}\n", command));
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
                eprintln!("  Cron scan failed: {}", e);
            }
        }
    }
    (yaml, count)
}


pub(crate) fn cmd_import(
    addr: &str,
    user: &str,
    name: Option<&str>,
    output: &Path,
    scan: &[String],
    verbose: bool,
) -> Result<(), String> {
    let machine_name = name.unwrap_or_else(|| {
        if addr == "localhost" || addr == "127.0.0.1" {
            "localhost"
        } else {
            addr.split('.').next().unwrap_or("imported")
        }
    });

    let machine = types::Machine {
        hostname: machine_name.to_string(),
        addr: addr.to_string(),
        user: user.to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };

    let scan_set: std::collections::HashSet<&str> = scan.iter().map(|s| s.as_str()).collect();

    let mut resources_yaml = String::new();
    let mut resource_count = 0;

    if scan_set.contains("packages") {
        let (yaml, count) = scan_packages(&machine, machine_name, verbose);
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
        r#"# Generated by: forjar import --addr {}
# Review and customize before applying.
version: "1.0"
name: imported-{}

machines:
  {}:
    hostname: {}
    addr: {}
    user: {}

resources:
{}
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#,
        addr, machine_name, machine_name, machine_name, addr, user, resources_yaml,
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
