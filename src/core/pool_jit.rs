//! Pool JIT runner configuration (#894, slice 2): a pure builder and validator
//! for an ephemeral just-in-time runner configuration. No network call, no
//! file write, no secret value.
//!
//! The configuration holds a reference NAME (an environment-variable style
//! identifier) saying where the registration token will be read from at run
//! time. It never holds a token, and it cannot name a path on disk for one.
//! There is no secret in any field, so the derived `Debug` and the `Display`
//! of [`JitConfigError`] have nothing secret to print.

use std::fmt;

/// Longest token lifetime the pool accepts: one hour.
pub const MAX_TOKEN_TTL_SECS: u64 = 3600;

/// Unvalidated request for one ephemeral runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JitRunnerRequest {
    pub labels: Vec<String>,
    pub ephemeral: bool,
    pub token_ttl_secs: u64,
    /// Name of the run-time source of the token, e.g. `RUNNER_JIT_TOKEN`.
    pub token_source: String,
}

/// A validated configuration. Only [`validate`] builds one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JitRunnerConfig {
    labels: Vec<String>,
    token_ttl_secs: u64,
    token_source: String,
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

/// Why a request was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JitConfigError {
    ZeroTtl,
    TtlTooLong { secs: u64 },
    EmptyLabelSet,
    BlankLabel,
    HostNameLabel { label: String },
    NotEphemeral,
    MissingTokenSource,
    TokenSourceIsPath { source: String },
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
            Self::HostNameLabel { label } => {
                write!(f, "label '{label}' is a host name pattern")
            }
            Self::NotEphemeral => write!(f, "runner is not ephemeral"),
            Self::MissingTokenSource => write!(f, "token source name is empty"),
            Self::TokenSourceIsPath { source } => {
                write!(f, "token source '{source}' names a path on disk")
            }
        }
    }
}

impl std::error::Error for JitConfigError {}

/// A label is a host name pattern when it contains `.`, `*` or `?`, or ends in
/// `-<digits>` (an instance ordinal such as `node-12`). Labels describe a
/// capability; they must not pin a runner to one machine.
pub fn is_host_name_label(label: &str) -> bool {
    if label.contains(['.', '*', '?']) {
        return true;
    }
    match label.rsplit_once('-') {
        Some((head, tail)) => {
            !head.is_empty() && !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit())
        }
        None => false,
    }
}

/// A token source names a path when it holds a separator, starts with `.` or
/// `~`, or carries a `file:` scheme.
fn names_a_path(source: &str) -> bool {
    source.contains(['/', '\\'])
        || source.starts_with(['.', '~'])
        || source.to_ascii_lowercase().starts_with("file:")
}

fn check_ttl(secs: u64) -> Result<(), JitConfigError> {
    match secs {
        0 => Err(JitConfigError::ZeroTtl),
        s if s > MAX_TOKEN_TTL_SECS => Err(JitConfigError::TtlTooLong { secs: s }),
        _ => Ok(()),
    }
}

fn check_label(l: &str) -> Result<(), JitConfigError> {
    if l.trim().is_empty() {
        return Err(JitConfigError::BlankLabel);
    }
    if is_host_name_label(l) {
        return Err(JitConfigError::HostNameLabel { label: l.into() });
    }
    Ok(())
}

fn check_labels(labels: &[String]) -> Result<(), JitConfigError> {
    if labels.is_empty() {
        return Err(JitConfigError::EmptyLabelSet);
    }
    labels.iter().try_for_each(|l| check_label(l))
}

fn check_source(source: &str) -> Result<(), JitConfigError> {
    if source.trim().is_empty() {
        return Err(JitConfigError::MissingTokenSource);
    }
    if names_a_path(source) {
        return Err(JitConfigError::TokenSourceIsPath {
            source: source.into(),
        });
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
mod tests {
    use super::*;

    fn req() -> JitRunnerRequest {
        JitRunnerRequest {
            labels: vec!["forjar-pool".into(), "linux-x64".into()],
            ephemeral: true,
            token_ttl_secs: MAX_TOKEN_TTL_SECS,
            token_source: "RUNNER_JIT_TOKEN".into(),
        }
    }

    #[test]
    fn accepts_a_valid_ephemeral_request() {
        let c = validate(&req()).expect("valid");
        assert_eq!(c.labels().len(), 2);
        assert_eq!(c.token_ttl_secs(), 3600);
        assert_eq!(c.token_source(), "RUNNER_JIT_TOKEN");
    }

    #[test]
    fn refuses_zero_ttl() {
        let r = JitRunnerRequest {
            token_ttl_secs: 0,
            ..req()
        };
        assert_eq!(validate(&r), Err(JitConfigError::ZeroTtl));
    }

    #[test]
    fn refuses_ttl_over_one_hour() {
        let r = JitRunnerRequest {
            token_ttl_secs: 3601,
            ..req()
        };
        assert_eq!(validate(&r), Err(JitConfigError::TtlTooLong { secs: 3601 }));
    }

    #[test]
    fn refuses_empty_label_set() {
        let r = JitRunnerRequest {
            labels: vec![],
            ..req()
        };
        assert_eq!(validate(&r), Err(JitConfigError::EmptyLabelSet));
    }

    #[test]
    fn refuses_blank_label() {
        let r = JitRunnerRequest {
            labels: vec!["  ".into()],
            ..req()
        };
        assert_eq!(validate(&r), Err(JitConfigError::BlankLabel));
    }

    #[test]
    fn refuses_host_name_labels() {
        for bad in ["build.example.com", "node-12", "ip-10-0-0-1", "host*"] {
            let r = JitRunnerRequest {
                labels: vec![bad.into()],
                ..req()
            };
            assert_eq!(
                validate(&r),
                Err(JitConfigError::HostNameLabel { label: bad.into() }),
                "{bad}"
            );
        }
        assert!(!is_host_name_label("linux-x64"));
    }

    #[test]
    fn refuses_non_ephemeral_runner() {
        let r = JitRunnerRequest {
            ephemeral: false,
            ..req()
        };
        assert_eq!(validate(&r), Err(JitConfigError::NotEphemeral));
    }

    #[test]
    fn refuses_empty_token_source() {
        let r = JitRunnerRequest {
            token_source: String::new(),
            ..req()
        };
        assert_eq!(validate(&r), Err(JitConfigError::MissingTokenSource));
    }

    #[test]
    fn refuses_token_source_naming_a_path() {
        for bad in ["/run/secrets/tok", "./tok", "~/tok", "a\\b", "file:tok"] {
            let r = JitRunnerRequest {
                token_source: bad.into(),
                ..req()
            };
            assert_eq!(
                validate(&r),
                Err(JitConfigError::TokenSourceIsPath { source: bad.into() }),
                "{bad}"
            );
        }
    }
}
