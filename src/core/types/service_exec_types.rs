//! PMAT-560: what a `service` unit EXECUTES, as declared state.
//!
//! # Why this type exists
//!
//! On yoga, 2026-09-15, `github-runner-ephemeral.service` was declared in
//! `machines/yoga/forjar-ephemeral.yaml` with a unit file whose `ExecStart`
//! named `run-ephemeral-docker.sh`. The unit systemd had LOADED executed
//! `/opt/github-runner-ephemeral/run-ephemeral-docker-v3.sh` — a file the repo
//! does not declare, hand-iterated on the box five days earlier — and every
//! check reported the service converged. A `service` resource asked two
//! questions of the host, `is-active` and `is-enabled`, and both were true of a
//! unit running code nobody had reviewed.
//!
//! "Declared" at the unit boundary said nothing about what executes. The layers
//! are unit → `ExecStart` → script bytes, and forjar checked only the first.
//!
//! # What the two fields pin
//!
//! `exec_start` is the path systemd must report as the unit's `ExecStart`
//! program — read from `systemctl show`, so a drop-in override or an edited
//! unit file is seen for what it is. `exec_sha256` is the sha256 of the file
//! at the path the unit ACTUALLY executes, never of the declared path: hashing
//! the declared path would confirm the file forjar just wrote and prove
//! nothing about the one systemd runs.
//!
//! Both are optional and independent. A `service` that declares neither keeps
//! the presence check it always had, and its observed digest does not move.
//!
//! Grouped into their own struct and `#[serde(flatten)]`-ed into `Resource`,
//! like `BackupSpec`: the YAML keys stay top level (`exec_start:`,
//! `exec_sha256:`), and `resource.rs` stays inside the 500-line health limit.

/// Declared executable identity for a `service` resource.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExecParity {
    /// Absolute path systemd must report as the unit's `ExecStart` program.
    #[serde(default)]
    pub exec_start: Option<String>,

    /// Lowercase hex sha256 of the file at the LIVE `ExecStart` path.
    #[serde(default)]
    pub exec_sha256: Option<String>,
}

impl ExecParity {
    /// True when the declaration asks forjar to look past the unit boundary.
    pub fn is_declared(&self) -> bool {
        self.exec_start.is_some() || self.exec_sha256.is_some()
    }
}

/// Length of a hex-encoded sha256 digest.
pub const SHA256_HEX_LEN: usize = 64;

/// True for exactly 64 lowercase hex characters — `sha256sum`'s own output
/// shape, which is what the generated check compares against verbatim.
///
/// Uppercase is refused rather than folded: the comparison on the host is a
/// string equality against `sha256sum`, so an uppercase declaration would be
/// permanently divergent, and refusing it at parse time is the honest place.
pub fn is_sha256_hex(s: &str) -> bool {
    s.len() == SHA256_HEX_LEN
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undeclared_is_not_declared() {
        assert!(!ExecParity::default().is_declared());
    }

    #[test]
    fn either_field_declares() {
        let p = ExecParity {
            exec_start: Some("/opt/x/run.sh".into()),
            exec_sha256: None,
        };
        assert!(p.is_declared());
        let h = ExecParity {
            exec_start: None,
            exec_sha256: Some("a".repeat(64)),
        };
        assert!(h.is_declared());
    }

    #[test]
    fn sha256_hex_is_exactly_64_lowercase_hex() {
        assert!(is_sha256_hex(&"0".repeat(64)));
        assert!(is_sha256_hex(
            "9de9af1a1739b9f8000000000000000000000000000000000000000000000000"
        ));
        assert!(!is_sha256_hex(&"0".repeat(63)));
        assert!(!is_sha256_hex(&"0".repeat(65)));
        assert!(!is_sha256_hex(&"A".repeat(64)), "uppercase is refused");
        assert!(!is_sha256_hex(&"g".repeat(64)));
        assert!(!is_sha256_hex(""));
    }

    #[test]
    fn flattened_keys_stay_top_level() {
        // The YAML shape is the contract with every manifest: `exec_start:`
        // beside `name:`, not nested under a sub-key.
        let r: crate::core::types::Resource = serde_yaml_ng::from_str(
            "type: service\nname: x\nexec_start: /opt/x/run.sh\nexec_sha256: 'aa'\n",
        )
        .expect("parses");
        assert_eq!(r.exec.exec_start.as_deref(), Some("/opt/x/run.sh"));
        assert_eq!(r.exec.exec_sha256.as_deref(), Some("aa"));
    }
}
