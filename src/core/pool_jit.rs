//! Pool JIT runner configuration (#894, slice 2): a pure validator for an
//! ephemeral just-in-time runner configuration. [`validate`] is the only
//! constructor of [`JitRunnerConfig`]; nothing here renders a request or a
//! shell line, makes a network call, writes a file or holds a secret value.
//!
//! The configuration holds a reference NAME saying where the registration
//! token will be read from at run time. The name must be an environment
//! variable identifier (`^[A-Z_][A-Z0-9_]*$`, at most 64 bytes), so a pasted
//! token, a path or a `$(...)` expression is refused. The errors for a bad
//! token source never echo the value, so neither the derived `Debug` nor the
//! `Display` of [`JitConfigError`] can print a token that was pasted by
//! mistake.
//!
//! Labels are limited to ASCII letters, digits and `- _ . :`, at most 256
//! bytes each, with no duplicates (case-insensitive), so a later caller can
//! join them with `,` into a runner `--labels` list without quoting.

use std::collections::HashSet;
use std::fmt;

/// Longest token lifetime the pool accepts: one hour.
pub const MAX_TOKEN_TTL_SECS: u64 = 3600;
/// Longest label GitHub accepts, in bytes.
pub const MAX_LABEL_LEN: usize = 256;
/// Longest token source name accepted, in bytes.
pub const MAX_TOKEN_SOURCE_LEN: usize = 64;

/// Unvalidated request for one ephemeral runner.
#[derive(Clone, PartialEq, Eq)]
pub struct JitRunnerRequest {
    pub labels: Vec<String>,
    pub ephemeral: bool,
    pub token_ttl_secs: u64,
    /// Name of the run-time source of the token, e.g. `RUNNER_JIT_TOKEN`.
    pub token_source: String,
}

impl fmt::Debug for JitRunnerRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JitRunnerRequest")
            .field("labels", &self.labels)
            .field("ephemeral", &self.ephemeral)
            .field("token_ttl_secs", &self.token_ttl_secs)
            .field("token_source", &"<redacted>")
            .finish()
    }
}

/// A validated configuration. Only [`validate`] builds one.
#[derive(Clone, PartialEq, Eq)]
pub struct JitRunnerConfig {
    labels: Vec<String>,
    token_ttl_secs: u64,
    token_source: String,
}

impl fmt::Debug for JitRunnerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JitRunnerConfig")
            .field("labels", &self.labels)
            .field("token_ttl_secs", &self.token_ttl_secs)
            .field("token_source", &"<redacted>")
            .finish()
    }
}

impl JitRunnerConfig {
    pub fn labels(&self) -> &[String] {
        &self.labels
    }
    pub fn token_ttl_secs(&self) -> u64 {
        self.token_ttl_secs
    }
    /// The reference name of the token source (never the token).
    pub fn token_source(&self) -> &str {
        &self.token_source
    }
}

/// Why a request was refused. Token source variants carry no value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JitConfigError {
    ZeroTtl,
    TtlTooLong { secs: u64 },
    EmptyLabelSet,
    BlankLabel,
    LabelHasComma { label: String },
    LabelHasWhitespaceOrControl { label: String },
    LabelForbiddenChar { label: String },
    LabelTooLong { len: usize },
    LabelLeadingDash { label: String },
    DuplicateLabel { label: String },
    NotEphemeral,
    MissingTokenSource,
    TokenSourceIsPath,
    TokenSourceTooLong { len: usize },
    TokenSourceNotEnvName,
}

impl fmt::Display for JitConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTtl => write!(f, "token lifetime is zero"),
            Self::TtlTooLong { secs } => {
                write!(f, "token lifetime {secs}s exceeds {MAX_TOKEN_TTL_SECS}s")
            }
            Self::EmptyLabelSet => write!(f, "label set is empty"),
            Self::BlankLabel => write!(f, "a label is blank"),
            Self::LabelHasComma { label } => write!(f, "label '{label}' contains a comma"),
            Self::LabelHasWhitespaceOrControl { label } => {
                write!(
                    f,
                    "label '{label}' contains whitespace or a control character"
                )
            }
            Self::LabelForbiddenChar { label } => {
                write!(
                    f,
                    "label '{label}' contains a character outside [A-Za-z0-9-_.:]"
                )
            }
            Self::LabelTooLong { len } => {
                write!(f, "a label is {len} bytes, over {MAX_LABEL_LEN}")
            }
            Self::LabelLeadingDash { label } => {
                write!(f, "label '{label}' starts with a dash")
            }
            Self::DuplicateLabel { label } => write!(f, "label '{label}' is repeated"),
            Self::NotEphemeral => write!(f, "runner is not ephemeral"),
            Self::MissingTokenSource => write!(f, "token source name is empty"),
            Self::TokenSourceIsPath => write!(f, "token source names a path on disk"),
            Self::TokenSourceTooLong { len } => {
                write!(
                    f,
                    "token source is {len} bytes, over {MAX_TOKEN_SOURCE_LEN}"
                )
            }
            Self::TokenSourceNotEnvName => {
                write!(f, "token source is not an environment variable name")
            }
        }
    }
}

impl std::error::Error for JitConfigError {}

/// A token source names a path when it holds a separator, starts with `.` or
/// `~`, or carries a `file:` scheme.
fn names_a_path(source: &str) -> bool {
    source.contains(['/', '\\'])
        || source.starts_with(['.', '~'])
        || source.to_ascii_lowercase().starts_with("file:")
}

fn is_env_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    match bytes.next() {
        Some(b) if b.is_ascii_uppercase() || b == b'_' => {}
        _ => return false,
    }
    bytes.all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

fn check_ttl(secs: u64) -> Result<(), JitConfigError> {
    match secs {
        0 => Err(JitConfigError::ZeroTtl),
        s if s > MAX_TOKEN_TTL_SECS => Err(JitConfigError::TtlTooLong { secs: s }),
        _ => Ok(()),
    }
}

fn check_label(l: &str) -> Result<(), JitConfigError> {
    let label = || l.to_string();
    if l.trim().is_empty() {
        return Err(JitConfigError::BlankLabel);
    }
    if l.contains(',') {
        return Err(JitConfigError::LabelHasComma { label: label() });
    }
    if l.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(JitConfigError::LabelHasWhitespaceOrControl { label: label() });
    }
    if l.len() > MAX_LABEL_LEN {
        return Err(JitConfigError::LabelTooLong { len: l.len() });
    }
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':');
    if !l.chars().all(allowed) {
        return Err(JitConfigError::LabelForbiddenChar { label: label() });
    }
    if l.starts_with('-') {
        return Err(JitConfigError::LabelLeadingDash { label: label() });
    }
    Ok(())
}

fn check_labels(labels: &[String]) -> Result<(), JitConfigError> {
    if labels.is_empty() {
        return Err(JitConfigError::EmptyLabelSet);
    }
    let mut seen = HashSet::new();
    for l in labels {
        check_label(l)?;
        if !seen.insert(l.to_ascii_lowercase()) {
            return Err(JitConfigError::DuplicateLabel { label: l.clone() });
        }
    }
    Ok(())
}

fn check_source(source: &str) -> Result<(), JitConfigError> {
    if source.trim().is_empty() {
        return Err(JitConfigError::MissingTokenSource);
    }
    if names_a_path(source) {
        return Err(JitConfigError::TokenSourceIsPath);
    }
    if source.len() > MAX_TOKEN_SOURCE_LEN {
        return Err(JitConfigError::TokenSourceTooLong { len: source.len() });
    }
    if !is_env_name(source) {
        return Err(JitConfigError::TokenSourceNotEnvName);
    }
    Ok(())
}

/// Validate a request into a configuration.
pub fn validate(r: &JitRunnerRequest) -> Result<JitRunnerConfig, JitConfigError> {
    if !r.ephemeral {
        return Err(JitConfigError::NotEphemeral);
    }
    check_ttl(r.token_ttl_secs)?;
    check_labels(&r.labels)?;
    check_source(&r.token_source)?;
    Ok(JitRunnerConfig {
        labels: r.labels.clone(),
        token_ttl_secs: r.token_ttl_secs,
        token_source: r.token_source.clone(),
    })
}

#[cfg(test)]
#[path = "pool_jit_tests.rs"]
mod tests;
