//! `forjar facts` — what is actually true about that host (#446).
//!
//! The ticket's list, verbatim: disk space, permissions, inodes, "who was
//! trying to write", and "a recurring issue on remote execution which is PATH
//! env var that might be incomplete or incorrect". Every one of those is a
//! MEASUREMENT of the target, and every one of them was, before this verb,
//! obtainable only by hand.
//!
//! One script, one round trip. The script is POSIX `sh` — no arrays, no
//! `[[`, no `local` — because the target's `/bin/sh` is as likely to be dash
//! or busybox as bash, and a facts verb that only works on hosts that already
//! have bash cannot answer "is bash installed here".

use std::collections::BTreeMap;

/// One real filesystem on the target.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct Disk {
    /// Mount point, e.g. `/var`.
    pub(crate) mount: String,
    /// Free space in 1024-byte blocks.
    pub(crate) avail_kb: u64,
    /// Percent of blocks used (0-100).
    pub(crate) use_pct: u32,
    /// Percent of inodes used (0-100). `0` where the filesystem has no inodes.
    pub(crate) inode_use_pct: u32,
}

/// Everything one `forjar facts` round trip measured.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub(crate) struct Facts {
    pub(crate) hostname: String,
    pub(crate) kernel: String,
    pub(crate) os: String,
    pub(crate) uptime_s: u64,
    pub(crate) user: String,
    /// `None` when the target did not answer or answered nonsense — never coerced
    /// to 0, which would read as root.
    pub(crate) uid: Option<u64>,
    /// Global IPv4 addresses, one per interface (`hostname -I`, else `ip`).
    pub(crate) ipv4: Vec<String>,
    pub(crate) groups: String,
    pub(crate) sudo: bool,
    pub(crate) shell: String,
    /// The login `PATH` as the transport sees it — the ticket's recurring bug.
    pub(crate) path: String,
    pub(crate) nproc: u64,
    pub(crate) mem_total_kb: u64,
    pub(crate) mem_avail_kb: u64,
    pub(crate) disks: Vec<Disk>,
    /// Executable name -> resolved path, or `None` when it is not installed.
    pub(crate) tools: BTreeMap<String, Option<String>>,
}

impl Facts {
    /// `ci (uid 1000)` — the identity forjar connects as, for a diagnostic;
    /// `ci (uid ?)` when the target did not report a usable uid.
    pub(crate) fn identity(&self) -> String {
        match self.uid {
            Some(uid) => format!("{} (uid {uid})", self.user),
            None => format!("{} (uid ?)", self.user),
        }
    }

    /// True when `<name>` resolved to a path on the target.
    pub(crate) fn has_tool(&self, name: &str) -> bool {
        matches!(self.tools.get(name), Some(Some(_)))
    }
}

/// The tools every provisioning run tends to need, probed in one pass.
pub(crate) const PROBED_TOOLS: &[&str] = &[
    "bash",
    "sh",
    "curl",
    "wget",
    "git",
    "python3",
    "systemctl",
    "apt-get",
    "dnf",
    "yum",
    "tar",
    "sudo",
];

/// The single POSIX `sh` script every fact comes from.
///
/// It writes nothing and reads nothing outside `/proc`, `/etc/os-release` and
/// `df` — `facts` is a measurement, so it must be safe to run on a host you
/// have not decided to change yet.
pub(crate) fn facts_script() -> String {
    format!(
        r#"echo "hostname=$(uname -n)"
echo "kernel=$(uname -sr)"
if [ -r /etc/os-release ]; then
  PRETTY_NAME=""
  . /etc/os-release
  echo "os=$PRETTY_NAME"
fi
if [ -r /proc/uptime ]; then
  echo "uptime_s=$(cut -d. -f1 /proc/uptime)"
fi
echo "user=$(id -un)"
echo "uid=$(id -u)"
echo "groups=$(id -Gn)"
if sudo -n true 2>/dev/null; then
  echo "sudo=yes"
else
  echo "sudo=no"
fi
echo "shell=$SHELL"
echo "path=$PATH"
for a in $(hostname -I 2>/dev/null || ip -o -4 addr show scope global 2>/dev/null | awk '{{ print $4 }}' | cut -d/ -f1); do
  echo "ipv4=$a"
done
echo "nproc=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 0)"
if [ -r /proc/meminfo ]; then
  echo "mem_total_kb=$(awk '/^MemTotal:/ {{ print $2 }}' /proc/meminfo)"
  echo "mem_avail_kb=$(awk '/^MemAvailable:/ {{ print $2 }}' /proc/meminfo)"
fi
df -Pk 2>/dev/null | tail -n +2 | while read -r fsrc fblocks fused favail fcap fmount; do
  skip=no
  case "$fsrc" in
    tmpfs|devtmpfs|overlay|squashfs|shm|none|udev|/dev/loop*) skip=yes ;;
  esac
  case "$fmount" in
    /dev/*|/proc/*|/sys/*|/run/*|/snap/*) skip=yes ;;
  esac
  if [ "$skip" = no ]; then
    finode=$(df -Pi "$fmount" 2>/dev/null | tail -n 1 | awk '{{ print $5 }}')
    echo "disk=$fmount:$favail:$fcap:$finode"
  fi
done
for probe in {tools}; do
  found=$(command -v "$probe" 2>/dev/null || echo "")
  if [ -n "$found" ]; then
    echo "tool=$probe:$found"
  else
    echo "tool=$probe:missing"
  fi
done
"#,
        tools = PROBED_TOOLS.join(" ")
    )
}

/// Parse `key=value` transcript lines into [`Facts`].
///
/// Unknown keys and malformed lines are SKIPPED, not fatal: a busybox target
/// that cannot answer one question must still yield the other twenty.
pub(crate) fn parse_facts(text: &str) -> Facts {
    let mut facts = Facts::default();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            assign(&mut facts, key.trim(), value.trim());
        }
    }
    facts
}

fn assign(facts: &mut Facts, key: &str, value: &str) {
    if assign_text(facts, key, value) || assign_number(facts, key, value) {
        return;
    }
    assign_row(facts, key, value);
}

fn assign_text(facts: &mut Facts, key: &str, value: &str) -> bool {
    let slot = match key {
        "hostname" => &mut facts.hostname,
        "kernel" => &mut facts.kernel,
        "os" => &mut facts.os,
        "user" => &mut facts.user,
        "groups" => &mut facts.groups,
        "shell" => &mut facts.shell,
        "path" => &mut facts.path,
        _ => return assign_flag(facts, key, value),
    };
    *slot = value.to_string();
    true
}

fn assign_flag(facts: &mut Facts, key: &str, value: &str) -> bool {
    if key != "sudo" {
        return false;
    }
    facts.sudo = value == "yes";
    true
}

/// A numeric key. An unparsable value is SKIPPED (the field keeps its
/// default), never coerced: a `uid` that failed to parse must not read as 0.
fn assign_number(facts: &mut Facts, key: &str, value: &str) -> bool {
    let slot: &mut u64 = match key {
        "uid" => {
            facts.uid = value.parse::<u64>().ok();
            return true;
        }
        "uptime_s" => &mut facts.uptime_s,
        "nproc" => &mut facts.nproc,
        "mem_total_kb" => &mut facts.mem_total_kb,
        "mem_avail_kb" => &mut facts.mem_avail_kb,
        _ => return false,
    };
    if let Ok(parsed) = value.parse::<u64>() {
        *slot = parsed;
    }
    true
}

fn assign_row(facts: &mut Facts, key: &str, value: &str) {
    match key {
        "disk" => facts.disks.extend(parse_disk(value)),
        "ipv4" => {
            if !value.is_empty() {
                facts.ipv4.push(value.to_string());
            }
        }
        "tool" => {
            if let Some((name, path)) = parse_tool(value) {
                facts.tools.insert(name, path);
            }
        }
        _ => {}
    }
}

/// `"/var:12345:62%:7%"` -> a [`Disk`]. Split from the RIGHT, because a mount
/// point may itself contain a colon.
pub(crate) fn parse_disk(value: &str) -> Option<Disk> {
    let (rest, inode) = value.rsplit_once(':')?;
    let (rest, capacity) = rest.rsplit_once(':')?;
    let (mount, avail) = rest.rsplit_once(':')?;
    if mount.is_empty() {
        return None;
    }
    Some(Disk {
        mount: mount.to_string(),
        avail_kb: avail.parse().unwrap_or(0),
        use_pct: percent(capacity),
        inode_use_pct: percent(inode),
    })
}

/// `df` prints `62%`, and `-` for a filesystem with no inodes.
fn percent(raw: &str) -> u32 {
    raw.trim().trim_end_matches('%').parse().unwrap_or(0)
}

/// `"curl:/usr/bin/curl"` -> `("curl", Some(path))`; `"wget:missing"` -> `None`.
pub(crate) fn parse_tool(value: &str) -> Option<(String, Option<String>)> {
    let (name, path) = value.split_once(':')?;
    if name.is_empty() {
        return None;
    }
    let resolved = if path == "missing" || path.is_empty() {
        None
    } else {
        Some(path.to_string())
    };
    Some((name.to_string(), resolved))
}

/// Gather facts from a machine over the shared transport.
pub(crate) fn gather(machine: &crate::core::types::Machine) -> Result<Facts, String> {
    let out = crate::transport::exec_script(machine, &facts_script())?;
    if out.exit_code != 0 && out.stdout.trim().is_empty() {
        return Err(format!(
            "facts script exited {} with no output: {}",
            out.exit_code,
            out.stderr.trim()
        ));
    }
    Ok(parse_facts(&out.stdout))
}

/// KB -> a human-sized string, so `mem_avail_kb: 4194304` reads as `4.0 GiB`.
pub(crate) fn human_kb(kb: u64) -> String {
    let mib = kb as f64 / 1024.0;
    if mib < 1024.0 {
        return format!("{mib:.0} MiB");
    }
    format!("{:.1} GiB", mib / 1024.0)
}

/// The readable report.
pub(crate) fn render(machine: &str, facts: &Facts) -> String {
    let mut out = format!("machine: {machine}\n");
    out.push_str(&format!("  host:    {} — {}\n", facts.hostname, facts.os));
    if !facts.ipv4.is_empty() {
        out.push_str(&format!("  ipv4:    {}\n", facts.ipv4.join(", ")));
    }
    out.push_str(&format!("  kernel:  {}\n", facts.kernel));
    out.push_str(&format!(
        "  user:    {} groups: {} sudo: {}\n",
        facts.identity(),
        facts.groups,
        if facts.sudo { "yes" } else { "no" }
    ));
    out.push_str(&format!("  shell:   {}\n", facts.shell));
    out.push_str(&format!("  PATH:    {}\n", facts.path));
    out.push_str(&format!(
        "  cpu/mem: {} cpu, {} of {} available\n",
        facts.nproc,
        human_kb(facts.mem_avail_kb),
        human_kb(facts.mem_total_kb)
    ));
    out.push_str(&format!("  uptime:  {}s\n", facts.uptime_s));
    out.push_str(&render_disks(facts));
    out.push_str(&render_tools(facts));
    out
}

fn render_disks(facts: &Facts) -> String {
    let mut out = String::from("  disks:\n");
    for disk in &facts.disks {
        out.push_str(&format!(
            "    {:<24} {:>10} free  {:>3}% used  {:>3}% inodes\n",
            disk.mount,
            human_kb(disk.avail_kb),
            disk.use_pct,
            disk.inode_use_pct
        ));
    }
    out
}

fn render_tools(facts: &Facts) -> String {
    let mut out = String::from("  tools:\n");
    for (name, path) in &facts.tools {
        let shown = path.clone().unwrap_or_else(|| "missing".to_string());
        out.push_str(&format!("    {name:<12} {shown}\n"));
    }
    out
}

/// #446: report what is true about a machine.
pub(crate) fn cmd_facts(
    file: &std::path::Path,
    machine_name: &str,
    json: bool,
) -> Result<(), String> {
    let config = super::helpers::parse_and_validate(file)?;
    let machine = super::exec::resolve_machine(&config, machine_name, file)?;
    let facts = gather(machine)?;
    if json {
        let text = serde_json::to_string_pretty(&facts).map_err(|e| format!("JSON error: {e}"))?;
        println!("{text}");
    } else {
        print!("{}", render(machine_name, &facts));
    }
    Ok(())
}
