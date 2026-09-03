//! `forjar doctor --machine <m>` — diagnose a HOST, read-only (#446).
//!
//! `forjar doctor` already checks the controller: is bash new enough, is ssh
//! installed, is the state directory sane. It said nothing about the machine
//! being provisioned, which is where the ticket's failures actually live:
//!
//! > An example scenario could be a permission denied on a curl request. Who is
//! > the owner? who was trying to write? what were the actual permissions in the
//! > destination directory?
//!
//! Each of those is one check below, answered from [`super::facts`] plus one
//! `stat`/`test -w` probe per declared destination. Nothing here writes to the
//! target — a diagnosis that changes the host it is diagnosing is not a
//! diagnosis.

use super::exec::shell_quote;
use super::facts::{Disk, Facts};
use crate::core::types::{ForjarConfig, Machine, Resource, ResourceType};
use std::path::Path;

/// Warn below this fraction of free blocks.
const DISK_WARN_FREE_PCT: u32 = 10;
/// Fail below this fraction of free blocks — an apply will not fit.
const DISK_FAIL_FREE_PCT: u32 = 2;
/// Warn below this many free 1024-byte blocks (1 GiB), whatever the percentage.
const DISK_WARN_FREE_KB: u64 = 1024 * 1024;
/// Warn below this fraction of free inodes. A full inode table fails writes
/// while `df -h` still shows space, which is the confusing form of this bug.
const INODE_WARN_FREE_PCT: u32 = 5;

/// The verdict of one check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Status {
    Pass,
    Warn,
    Fail,
}

impl Status {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Warn => "warn",
            Status::Fail => "FAIL",
        }
    }
}

/// One named observation about the target.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct Check {
    pub(crate) name: String,
    pub(crate) status: Status,
    pub(crate) detail: String,
}

impl Check {
    pub(crate) fn new(name: &str, status: Status, detail: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            status,
            detail: detail.into(),
        }
    }
}

/// The PATH entries whose absence is the ticket's named recurring bug: a
/// non-interactive SSH shell whose PATH lacks them cannot find `systemctl` or
/// anything installed under `/usr/local`, and the failure reads as "command
/// not found" rather than as a PATH problem.
const REQUIRED_PATH_DIRS: &[&str] = &["/usr/local/bin", "/usr/sbin"];

/// Is the remote PATH complete? The PATH itself is in the detail either way —
/// it is the fact the operator needs whether or not this check passes.
pub(crate) fn path_check(path: &str) -> Check {
    let entries: Vec<&str> = path.split(':').collect();
    let missing: Vec<&str> = REQUIRED_PATH_DIRS
        .iter()
        .copied()
        .filter(|d| !entries.contains(d))
        .collect();
    if missing.is_empty() {
        return Check::new("PATH", Status::Pass, format!("PATH={path}"));
    }
    Check::new(
        "PATH",
        Status::Warn,
        format!("PATH={path} — missing {}", missing.join(", ")),
    )
}

/// The worst disk decides the verdict; the detail names it.
pub(crate) fn disk_check(disks: &[Disk]) -> Check {
    if disks.is_empty() {
        return Check::new("disk", Status::Warn, "no filesystems reported by df");
    }
    let worst = disks
        .iter()
        .max_by_key(|d| d.use_pct)
        .expect("disks is non-empty");
    let free_pct = 100u32.saturating_sub(worst.use_pct);
    let detail = format!(
        "{} is {}% used, {} free",
        worst.mount,
        worst.use_pct,
        super::facts::human_kb(worst.avail_kb)
    );
    if free_pct < DISK_FAIL_FREE_PCT {
        return Check::new("disk", Status::Fail, detail);
    }
    if free_pct < DISK_WARN_FREE_PCT || worst.avail_kb < DISK_WARN_FREE_KB {
        return Check::new("disk", Status::Warn, detail);
    }
    Check::new("disk", Status::Pass, detail)
}

/// Inodes get their own check because they fail independently of blocks.
pub(crate) fn inode_check(disks: &[Disk]) -> Check {
    let tight: Vec<String> = disks
        .iter()
        .filter(|d| 100u32.saturating_sub(d.inode_use_pct) < INODE_WARN_FREE_PCT)
        .map(|d| format!("{} at {}% inodes", d.mount, d.inode_use_pct))
        .collect();
    if tight.is_empty() {
        return Check::new("inodes", Status::Pass, "inode tables have headroom");
    }
    Check::new("inodes", Status::Warn, tight.join("; "))
}

/// The directory a `file` resource writes into.
pub(crate) fn parent_dir(path: &str) -> String {
    match Path::new(path).parent() {
        Some(p) if p.as_os_str().is_empty() => ".".to_string(),
        Some(p) => p.display().to_string(),
        None => "/".to_string(),
    }
}

/// True when this resource is declared on `machine`.
fn targets(resource: &Resource, machine: &str) -> bool {
    resource.machine.iter().any(|m| m == machine)
}

/// Every destination directory this machine's `file` resources write into,
/// in declaration order, without repeats.
pub(crate) fn destination_dirs(config: &ForjarConfig, machine: &str) -> Vec<String> {
    let mut dirs: Vec<String> = Vec::new();
    for resource in config.resources.values() {
        if resource.resource_type != ResourceType::File || !targets(resource, machine) {
            continue;
        }
        let Some(path) = resource.path.as_deref() else {
            continue;
        };
        let dir = parent_dir(path);
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

/// What one `stat`/`test -w` probe found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirStat {
    pub(crate) path: String,
    pub(crate) exists: bool,
    pub(crate) owner: String,
    pub(crate) group: String,
    pub(crate) mode: String,
    pub(crate) writable: bool,
}

/// A read-only probe of every destination directory, in one round trip.
pub(crate) fn dir_probe_script(dirs: &[String]) -> String {
    let quoted: Vec<String> = dirs.iter().map(|d| shell_quote(d)).collect();
    format!(
        r#"for probe_dir in {}; do
  if [ -d "$probe_dir" ]; then
    probe_stat=$(stat -c '%U:%G:%a' "$probe_dir" 2>/dev/null || echo '?:?:?')
    if [ -w "$probe_dir" ]; then
      probe_w=yes
    else
      probe_w=no
    fi
    echo "dirstat=$probe_dir|yes|$probe_stat|$probe_w"
  else
    echo "dirstat=$probe_dir|no|?:?:?|no"
  fi
done
"#,
        quoted.join(" ")
    )
}

/// Parse one `dirstat=` transcript line.
pub(crate) fn parse_dir_stat(line: &str) -> Option<DirStat> {
    let value = line.trim().strip_prefix("dirstat=")?;
    let fields: Vec<&str> = value.split('|').collect();
    if fields.len() != 4 {
        return None;
    }
    let ownership: Vec<&str> = fields[2].split(':').collect();
    Some(DirStat {
        path: fields[0].to_string(),
        exists: fields[1] == "yes",
        owner: ownership.first().unwrap_or(&"?").to_string(),
        group: ownership.get(1).unwrap_or(&"?").to_string(),
        mode: ownership.get(2).unwrap_or(&"?").to_string(),
        writable: fields[3] == "yes",
    })
}

/// The sentence the ticket asks for: owner, group, mode, and the identity
/// forjar connects as, in one line, because those four together are the whole
/// answer to "why was this a permission denied".
pub(crate) fn permission_detail(stat: &DirStat, facts: &Facts) -> String {
    format!(
        "{} is owned by {}:{} mode {}; forjar connects as {} — sudo: {}",
        stat.path,
        stat.owner,
        stat.group,
        stat.mode,
        facts.identity(),
        if facts.sudo { "yes" } else { "no" }
    )
}

/// Verdict for one destination directory.
pub(crate) fn dir_check(stat: &DirStat, facts: &Facts) -> Check {
    if !stat.exists {
        return Check::new(
            "destination",
            Status::Warn,
            format!(
                "{} does not exist yet; forjar connects as {}",
                stat.path,
                facts.identity()
            ),
        );
    }
    let status = if stat.writable {
        Status::Pass
    } else {
        Status::Fail
    };
    Check::new("destination", status, permission_detail(stat, facts))
}

/// An executable the machine's resources will need. `candidates` is an
/// any-of: `curl` OR `wget` downloads a release, and demanding both would
/// report a failure on a host that works.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolNeed {
    pub(crate) candidates: Vec<String>,
    pub(crate) why: String,
}

impl ToolNeed {
    fn new(candidates: &[&str], why: String) -> Self {
        Self {
            candidates: candidates.iter().map(|c| (*c).to_string()).collect(),
            why,
        }
    }
}

/// `(provider, executable)` — a `package` resource's provider needs its
/// executable on the TARGET. Extend the table, not a match.
const PACKAGE_TOOLS: &[(&str, &str)] = &[
    ("apt", "apt-get"),
    ("apt-get", "apt-get"),
    ("dnf", "dnf"),
    ("yum", "yum"),
    ("zypper", "zypper"),
    ("pacman", "pacman"),
    ("apk", "apk"),
    ("brew", "brew"),
    ("snap", "snap"),
    ("cargo", "cargo"),
    ("uv", "uv"),
    ("pip", "pip3"),
    ("pip3", "pip3"),
];

fn package_tool(resource: &Resource) -> Option<&'static str> {
    let provider = resource.provider.as_deref()?;
    PACKAGE_TOOLS
        .iter()
        .find(|(p, _)| *p == provider)
        .map(|(_, tool)| *tool)
}

/// A `source:` that is a git remote needs git on the TARGET, because that is
/// where the clone runs.
fn git_need(id: &str, resource: &Resource) -> Option<ToolNeed> {
    let source = resource.source.as_deref().unwrap_or("");
    let is_git =
        source.ends_with(".git") || source.starts_with("git@") || source.starts_with("git://");
    is_git.then(|| ToolNeed::new(&["git"], format!("git source on resource '{id}'")))
}

fn need_for(id: &str, resource: &Resource) -> Option<ToolNeed> {
    match resource.resource_type {
        ResourceType::Package => package_tool(resource)
            .map(|tool| ToolNeed::new(&[tool], format!("package resource '{id}'"))),
        ResourceType::Service => Some(ToolNeed::new(
            &["systemctl"],
            format!("service resource '{id}'"),
        )),
        ResourceType::GithubRelease => Some(ToolNeed::new(
            &["curl", "wget"],
            format!("github_release resource '{id}'"),
        )),
        _ => git_need(id, resource),
    }
}

/// Every executable this machine's declared resources need.
pub(crate) fn tool_needs(config: &ForjarConfig, machine: &str) -> Vec<ToolNeed> {
    let mut needs: Vec<ToolNeed> = Vec::new();
    for (id, resource) in &config.resources {
        if !targets(resource, machine) {
            continue;
        }
        let Some(need) = need_for(id, resource) else {
            continue;
        };
        if !needs.iter().any(|n| n.candidates == need.candidates) {
            needs.push(need);
        }
    }
    needs
}

/// Verdict for one tool requirement.
pub(crate) fn tool_check(need: &ToolNeed, facts: &Facts) -> Check {
    let found = need.candidates.iter().find(|c| facts.has_tool(c));
    let names = need.candidates.join(" or ");
    match found {
        Some(name) => Check::new(
            "tool",
            Status::Pass,
            format!(
                "{name} at {} ({})",
                facts
                    .tools
                    .get(name)
                    .and_then(Clone::clone)
                    .unwrap_or_default(),
                need.why
            ),
        ),
        None => Check::new(
            "tool",
            Status::Fail,
            format!("{names} not installed — needed by {}", need.why),
        ),
    }
}

/// `5 checks: 3 pass, 1 warn, 1 fail`.
pub(crate) fn summary_line(checks: &[Check]) -> String {
    let count = |s: Status| checks.iter().filter(|c| c.status == s).count();
    format!(
        "{} checks: {} pass, {} warn, {} fail",
        checks.len(),
        count(Status::Pass),
        count(Status::Warn),
        count(Status::Fail)
    )
}

/// The report an operator reads.
pub(crate) fn render(machine: &str, addr: &str, checks: &[Check]) -> String {
    let mut out = format!("machine: {machine} ({addr})\n");
    for check in checks {
        out.push_str(&format!(
            "  [{:>4}] {:<12} {}\n",
            check.status.label(),
            check.name,
            check.detail
        ));
    }
    out.push_str(&summary_line(checks));
    out.push('\n');
    out
}

/// Non-zero iff any check FAILED. Warnings are information, not a gate.
pub(crate) fn verdict(machine: &str, checks: &[Check]) -> Result<(), String> {
    let failed = checks.iter().filter(|c| c.status == Status::Fail).count();
    if failed == 0 {
        return Ok(());
    }
    Err(format!(
        "machine '{machine}': {failed} failing check(s) — {}",
        summary_line(checks)
    ))
}

fn reachable_check(machine: &Machine) -> Result<Check, String> {
    let out = crate::transport::exec_script(machine, "true")
        .map_err(|e| format!("cannot reach machine: {e}"))?;
    if out.exit_code != 0 {
        return Err(format!(
            "cannot reach machine: `true` exited {}: {}",
            out.exit_code,
            out.stderr.trim()
        ));
    }
    Ok(Check::new("reachable", Status::Pass, "`true` exited 0"))
}

fn facts_check(facts: &Facts) -> Check {
    Check::new(
        "facts",
        Status::Pass,
        format!("{} — {} — {}", facts.hostname, facts.os, facts.kernel),
    )
}

/// Probe every destination directory and turn each answer into a check.
fn dir_checks(
    machine: &Machine,
    config: &ForjarConfig,
    machine_name: &str,
    facts: &Facts,
) -> Result<Vec<Check>, String> {
    let dirs = destination_dirs(config, machine_name);
    if dirs.is_empty() {
        return Ok(Vec::new());
    }
    let out = crate::transport::exec_script(machine, &dir_probe_script(&dirs))?;
    Ok(out
        .stdout
        .lines()
        .filter_map(parse_dir_stat)
        .map(|stat| dir_check(&stat, facts))
        .collect())
}

fn emit(machine_name: &str, addr: &str, checks: &[Check], json: bool) -> Result<(), String> {
    if json {
        let text = serde_json::to_string_pretty(checks).map_err(|e| format!("JSON error: {e}"))?;
        println!("{text}");
        return Ok(());
    }
    print!("{}", render(machine_name, addr, checks));
    Ok(())
}

/// #446: diagnose one machine. Read-only, always.
pub(crate) fn cmd_doctor_machine(
    file: &Path,
    machine_name: &str,
    json: bool,
) -> Result<(), String> {
    let config = super::helpers::parse_and_validate(file)?;
    let machine = super::exec::resolve_machine(&config, machine_name, file)?;

    let mut checks = vec![reachable_check(machine)?];
    let facts = super::facts::gather(machine)?;
    checks.push(facts_check(&facts));
    checks.push(path_check(&facts.path));
    checks.push(disk_check(&facts.disks));
    checks.push(inode_check(&facts.disks));
    checks.extend(dir_checks(machine, &config, machine_name, &facts)?);
    checks.extend(
        tool_needs(&config, machine_name)
            .iter()
            .map(|need| tool_check(need, &facts)),
    );

    emit(machine_name, &machine.addr, &checks, json)?;
    verdict(machine_name, &checks)
}
