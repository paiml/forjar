//! FJ-LOCK-001 C3/C5 (#923, #925): who holds a lock, and is that holder alive.
//!
//! A bare PID is not an identity: PIDs are reused, and a state dir on a shared
//! mount can be locked from another host or from before a reboot. The lock
//! content therefore carries a uuid `id` (what release and `--force-unlock`
//! match on), `host`, `boot_id`, `pid` and the process `starttime` (clock ticks
//! since boot, `/proc/<pid>/stat` field 22), plus `command` and `started_at`
//! for the operator reading the refusal.

use std::path::Path;

/// The identity written into a lock file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LockInfo {
    /// Random uuid; release and `--force-unlock <ID>` match on this.
    pub id: String,
    pub host: String,
    pub boot_id: String,
    pub pid: u32,
    /// `/proc/<pid>/stat` starttime of the holder; empty when unreadable.
    pub starttime: String,
    pub command: String,
    pub started_at: String,
}

/// What the lock's recorded identity says about its holder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holder {
    /// A live process on this host and boot.
    LiveLocal,
    /// Recorded by another host: this host cannot tell, so it is never reaped.
    Remote,
    /// Process gone, PID reused, or host rebooted since: safe to reap.
    Dead,
}

impl LockInfo {
    /// The identity of the current process.
    pub fn current() -> Self {
        let pid = std::process::id();
        LockInfo {
            id: new_id(),
            host: local_host(),
            boot_id: local_boot_id(),
            pid,
            starttime: proc_starttime(pid).unwrap_or_default(),
            command: std::env::args()
                .collect::<Vec<_>>()
                .join(" ")
                .replace('\n', " "),
            started_at: crate::tripwire::eventlog::now_iso8601(),
        }
    }

    /// `key: value` lines; `pid:` stays a top-level key for older readers.
    pub fn render(&self) -> String {
        format!(
            "id: {}\nhost: {}\nboot_id: {}\npid: {}\nstarttime: {}\ncommand: {}\nstarted_at: {}\n",
            self.id,
            self.host,
            self.boot_id,
            self.pid,
            self.starttime,
            self.command,
            self.started_at
        )
    }

    /// Parse lock content. Legacy `pid:`/`started_at:` locks parse with the
    /// remaining fields empty; unparseable content yields `pid == 0`.
    pub fn parse(content: &str) -> Self {
        let mut info = LockInfo::default();
        for line in content.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim().to_string();
            match key.trim() {
                "id" => info.id = value,
                "host" => info.host = value,
                "boot_id" => info.boot_id = value,
                "pid" => info.pid = value.parse().unwrap_or(0),
                "starttime" => info.starttime = value,
                "command" => info.command = value,
                "started_at" => info.started_at = value,
                _ => {}
            }
        }
        info
    }

    /// Classify the holder against this process's host, boot and `/proc`.
    pub fn holder(&self) -> Holder {
        self.holder_with(is_pid_running)
    }

    /// [`LockInfo::holder`] with the liveness probe injected.
    pub fn holder_with(&self, is_running: impl Fn(u32) -> bool) -> Holder {
        if !self.host.is_empty() && self.host != local_host() {
            return Holder::Remote;
        }
        if !self.boot_id.is_empty() && self.boot_id != local_boot_id() {
            return Holder::Dead;
        }
        if self.pid == 0 || !is_running(self.pid) {
            return Holder::Dead;
        }
        match (self.starttime.as_str(), proc_starttime(self.pid)) {
            ("", _) => Holder::LiveLocal,
            (recorded, Some(now)) if recorded == now => Holder::LiveLocal,
            _ => Holder::Dead,
        }
    }

    /// True when this process wrote the lock (same pid on this host).
    pub fn owned_by_this_process(&self) -> bool {
        self.pid == std::process::id() && (self.host.is_empty() || self.host == local_host())
    }

    /// One line for an error: every field an operator needs to decide.
    pub fn describe(&self) -> String {
        format!(
            "id={} host={} pid={} command=[{}] started_at={}",
            or_unknown(&self.id),
            or_unknown(&self.host),
            self.pid,
            or_unknown(&self.command),
            or_unknown(&self.started_at)
        )
    }
}

fn or_unknown(s: &str) -> &str {
    if s.is_empty() {
        "unknown"
    } else {
        s
    }
}

/// Read and parse a lock file's identity; `None` when absent or unreadable.
pub fn read_lock_info(lock_path: &Path) -> Option<LockInfo> {
    std::fs::read_to_string(lock_path)
        .ok()
        .map(|c| LockInfo::parse(&c))
}

/// Check if a PID is still running (Linux-specific: /proc/<pid> exists).
pub(super) fn is_pid_running(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

fn local_host() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn local_boot_id() -> String {
    std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Field 22 of `/proc/<pid>/stat`, parsed after the last `)` so a command name
/// containing spaces or parentheses cannot shift the fields.
fn proc_starttime(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = stat.rsplit_once(')')?.1;
    // After ")": state is field 3, so starttime (22) is index 19 here.
    rest.split_whitespace().nth(19).map(str::to_string)
}

/// A random v4-shaped uuid from the kernel's CSPRNG (no new dependency).
fn new_id() -> String {
    use std::io::Read;
    let mut b = [0u8; 16];
    let ok = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .is_ok();
    if !ok {
        // Fall back to time and pid: unique enough to tell two holders apart.
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        b = (t ^ (u128::from(std::process::id()) << 96)).to_le_bytes();
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}
