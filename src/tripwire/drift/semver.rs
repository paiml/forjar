//! `MAJOR.MINOR.PATCH`, read the way forjar#613 reads a live `--version`.
//! Split out of `version_pin.rs` to keep that file under the 500-line ratchet.

use std::fmt;

/// `MAJOR.MINOR.PATCH`. Pre-release and build suffixes are not compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Semver {
    /// Major.
    pub major: u64,
    /// Minor.
    pub minor: u64,
    /// Patch.
    pub patch: u64,
}

impl fmt::Display for Semver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The first `MAJOR.MINOR.PATCH` in `text`, with an optional leading `v`.
///
/// It reads version output in the shapes found in the wild:
/// `ollama version is 0.34.2`, `forjar 1.32.0`, `v0.34.2`, and ollama's
/// `Warning: could not connect to a running Ollama instance` followed by
/// `Warning: client version is 0.34.2`. A number that is part of a longer
/// dotted run (`10.42.0.11`) or glued to letters (`x86_64`) is not a version.
/// `None` means there was no version in the output. It is never taken as a match.
pub fn parse_semver(text: &str) -> Option<Semver> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() && starts_token(bytes, i) {
            if let Some((v, end)) = triple_at(bytes, i) {
                if !continues_dotted(bytes, end) {
                    return Some(v);
                }
            }
        }
        i += 1;
    }
    None
}

/// A version starts a token: at the start of the text, after a non-alphanumeric
/// byte that is not a dot, or after a `v` that itself starts a token.
fn starts_token(bytes: &[u8], i: usize) -> bool {
    let boundary = |j: usize| {
        j == 0
            || !(bytes[j - 1].is_ascii_alphanumeric()
                || bytes[j - 1] == b'.'
                || bytes[j - 1] == b'_')
    };
    if boundary(i) {
        return true;
    }
    matches!(bytes[i - 1], b'v' | b'V') && boundary(i - 1)
}

/// `(version, index just past it)` for `N.N.N` starting at `i`.
fn triple_at(bytes: &[u8], mut i: usize) -> Option<(Semver, usize)> {
    let mut parts = [0u64; 3];
    for (n, part) in parts.iter_mut().enumerate() {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
        *part = std::str::from_utf8(&bytes[start..i]).ok()?.parse().ok()?;
        if n < 2 {
            if bytes.get(i) != Some(&b'.') {
                return None;
            }
            i += 1;
        }
    }
    Some((
        Semver {
            major: parts[0],
            minor: parts[1],
            patch: parts[2],
        },
        i,
    ))
}

/// `1.2.3.4` is not a semver, and neither is its `1.2.3`.
fn continues_dotted(bytes: &[u8], end: usize) -> bool {
    bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit)
}
