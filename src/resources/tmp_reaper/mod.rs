//! forjar#847: `tmp_reaper` — delete files older than `max_age` under `path`.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::verdict;

/// A validated reaper declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    /// Directory to reap, absolute, never `/`.
    pub path: String,
    /// Age in minutes beyond which an entry is removed.
    pub max_age_min: u64,
    /// Optional `find -name` glob (no `/`).
    pub pattern: Option<String>,
}

/// `7d`, `12h`, `30m`, `2w` → minutes.
///
/// # Errors
///
/// Anything else, and zero.
pub fn parse_age(s: &str) -> Result<u64, String> {
    let bad = || format!("max_age {s:?} must be a whole number plus m, h, d or w (e.g. 7d)");
    let unit = s.chars().last().ok_or_else(bad)?;
    let mult: u64 = match unit {
        'm' => 1,
        'h' => 60,
        'd' => 1_440,
        'w' => 10_080,
        _ => return Err(bad()),
    };
    let digits = &s[..s.len() - 1];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let n: u64 = digits.parse().map_err(|_| bad())?;
    if n == 0 {
        return Err(format!("max_age {s:?} must be greater than zero"));
    }
    n.checked_mul(mult).ok_or_else(bad)
}

/// Validate a resource's declaration.
///
/// # Errors
///
/// Names the first problem. A path of `/` (or only slashes), an empty or
/// relative path, and any `..` segment are refused.
pub fn spec_of(r: &Resource) -> Result<Spec, String> {
    let raw = r.path.as_deref().ok_or("path is required")?;
    if raw.trim().is_empty() {
        return Err("path must not be empty".into());
    }
    if !raw.starts_with('/') {
        return Err(format!("path {raw:?} must be absolute"));
    }
    let path = match raw.trim_end_matches('/') {
        "" => return Err("path must not be / (refusing to reap the root)".into()),
        p => p.to_string(),
    };
    if path.split('/').any(|seg| seg == "..") {
        return Err(format!("path {raw:?} must not contain .."));
    }
    let age = r.max_age.as_deref().ok_or("max_age is required")?;
    let pattern = match r.pattern.as_deref() {
        None => None,
        Some("") => return Err("pattern must not be empty".into()),
        Some(p) if p.contains(['/', '\0']) => {
            return Err(format!("pattern {p:?} must not contain /"));
        }
        Some(p) => Some(p.to_string()),
    };
    Ok(Spec {
        path,
        max_age_min: parse_age(age)?,
        pattern,
    })
}

/// Shell that refuses a root, a symlinked path, and exits 0 quietly when the
/// directory is absent (`$P` is set; `$ABSENT` is run for a missing dir).
fn guard(spec: &Spec, on_absent: &str) -> String {
    format!(
        "P={p}\n\
         if [ -L \"$P\" ]; then echo \"refusing: $P is a symlink\" >&2; exit 2; fi\n\
         if [ ! -d \"$P\" ]; then {on_absent}; fi\n\
         if [ \"$(cd \"$P\" && pwd -P)\" = / ]; then echo 'refusing: path resolves to /' >&2; exit 2; fi",
        p = sh_squote(&spec.path),
    )
}

fn name_arg(spec: &Spec) -> String {
    spec.pattern
        .as_deref()
        .map_or_else(String::new, |p| format!(" -name {}", sh_squote(p)))
}

fn broken(msg: &str) -> String {
    format!("echo {} >&2; exit 2", sh_squote(msg))
}

/// Exit 0 iff nothing is older than `max_age`.
pub fn check_script(r: &Resource) -> String {
    let spec = match spec_of(r) {
        Ok(s) => s,
        Err(e) => return broken(&format!("tmp_reaper: {e}")),
    };
    // Select exactly what `apply_script` removes: old files and symlinks
    // (matching the pattern), plus old EMPTY directories when no pattern.
    let dirs = if spec.pattern.is_none() {
        " -o -type d -empty"
    } else {
        ""
    };
    let cond = format!(
        "[ -z \"$(find \"$P\" -xdev -mindepth 1 -mmin +{} \\( \\( -type f -o -type l \\){}{dirs} \\) -print -quit)\" ]",
        spec.max_age_min,
        name_arg(&spec),
    );
    let verdict = verdict::single(
        &cond,
        "tmp_reaper: nothing stale",
        "tmp_reaper: stale entries",
    );
    format!(
        "{}\n{verdict}",
        guard(&spec, "echo 'tmp_reaper: path absent'; exit 0")
    )
}

/// Delete, then report files and inodes removed.
///
/// `find` does not follow symlinks (`-P`), stays on one filesystem (`-xdev`)
/// and unlinks a symlink itself, never its target.
pub fn apply_script(r: &Resource) -> String {
    let spec = match spec_of(r) {
        Ok(s) => s,
        Err(e) => return broken(&format!("tmp_reaper: {e}")),
    };
    let m = spec.max_age_min;
    let name = name_arg(&spec);
    let dirs = if spec.pattern.is_some() {
        "D=0".to_string()
    } else {
        format!(
            "D=$(find \"$P\" -xdev -mindepth 1 -type d -empty -mmin +{m} -print0 -delete | tr -cd '\\0' | wc -c)"
        )
    };
    format!(
        "set -euo pipefail\n{}\n\
         F=$(find \"$P\" -xdev -mindepth 1 \\( -type f -o -type l \\) -mmin +{m}{name} -print0 -delete | tr -cd '\\0' | wc -c)\n\
         {dirs}\n\
         echo \"removed $F file(s), $((F + D)) inode(s)\"",
        guard(&spec, "echo 'removed 0 file(s), 0 inode(s)'; exit 0"),
    )
}

/// Observable state for hashing: how many entries are past `max_age`.
pub fn state_query_script(r: &Resource) -> String {
    let spec = match spec_of(r) {
        Ok(s) => s,
        Err(e) => return broken(&format!("tmp_reaper: {e}")),
    };
    format!(
        "{}\necho \"stale=$(find \"$P\" -xdev -mindepth 1 -mmin +{} \\( \\( -type f -o -type l \\){}{} \\) -print0 | tr -cd '\\0' | wc -c)\"",
        guard(&spec, "echo 'stale=absent'; exit 0"),
        spec.max_age_min,
        name_arg(&spec),
        if spec.pattern.is_none() { " -o -type d -empty" } else { "" },
    )
}

#[cfg(test)]
mod tests;
