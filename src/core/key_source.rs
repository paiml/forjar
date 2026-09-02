//! E13: naming key material without putting it on `argv`.
//!
//! `forjar lock-sign --key <KEY>` used to take the signing key as a bare
//! string that was hashed verbatim, so the only working invocation put the
//! secret itself on the command line — where `ps` hands it to every other
//! user on the host. The help even claimed "path to key file or inline"; the
//! file half was never implemented.
//!
//! This module is the one place that turns a `--key`-shaped argument into key
//! material. Two indirect forms keep the secret off `argv`:
//!
//! * `file:<PATH>` — the key is the file's contents (surrounding whitespace
//!   trimmed, so a trailing newline from `echo` is not part of the key).
//! * `env:<VAR>` — the key is the environment variable's value (also
//!   trimmed). The environment is not in `ps` output.
//!
//! Anything else is taken as an inline literal, which still works so that
//! existing signatures keep verifying, but warns on every use and is removed
//! in [`INLINE_KEY_REMOVAL_VERSION`].
//!
//! forjar already pipes `script:` bodies to remote shells over stdin
//! specifically so they never appear in `ps`; this gives key material the
//! same treatment.

use std::path::{Path, PathBuf};

/// Release that drops the inline (`argv`) form of key arguments.
pub const INLINE_KEY_REMOVAL_VERSION: &str = "2.0.0";

/// Prefix selecting the file form.
const FILE_PREFIX: &str = "file:";
/// Prefix selecting the environment form.
const ENV_PREFIX: &str = "env:";

/// Help text for every CLI flag that takes signing key material.
///
/// Shared so that no flag can drift back to documenting a capability the
/// resolver does not have — a lying help string is how the secret ended up
/// in `ps` in the first place.
pub const KEY_ARG_HELP: &str = "Signing key, named indirectly so it stays out of `ps`: \
     `file:<PATH>` (key is the file's contents) or `env:<VAR>` (key is the variable's value). \
     A bare literal is still accepted but is DEPRECATED — it is visible to every user on this \
     host — and is removed in forjar 2.0.0";

/// Where a `--key`-shaped argument says its key material lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySource {
    /// `file:<PATH>` — read the key from a file.
    File(PathBuf),
    /// `env:<VAR>` — read the key from an environment variable.
    Env(String),
    /// A literal typed on the command line. Deprecated: visible in `ps`.
    Inline,
}

/// Classify a key argument without reading anything.
pub fn classify(spec: &str) -> KeySource {
    if let Some(rest) = spec.strip_prefix(FILE_PREFIX) {
        KeySource::File(PathBuf::from(rest))
    } else if let Some(rest) = spec.strip_prefix(ENV_PREFIX) {
        KeySource::Env(rest.to_string())
    } else {
        KeySource::Inline
    }
}

/// Resolve a key argument to the key material itself.
///
/// `flag` is the CLI flag being resolved (`"--key"`, `"--old-key"`, …); it
/// appears in errors and in the deprecation warning so an operator can tell
/// which of several key flags is at fault. Errors never quote the key.
pub fn resolve(spec: &str, flag: &str) -> Result<String, String> {
    match classify(spec) {
        KeySource::File(path) => read_key_file(&path, flag),
        KeySource::Env(var) => read_key_env(&var, flag),
        KeySource::Inline => {
            warn_inline(flag);
            Ok(spec.to_string())
        }
    }
}

/// Resolve an optional key argument, leaving `None` alone.
pub fn resolve_opt(spec: Option<&str>, flag: &str) -> Result<Option<String>, String> {
    match spec {
        Some(s) => resolve(s, flag).map(Some),
        None => Ok(None),
    }
}

/// Read key material from a file, refusing anything that is not a key.
///
/// An unreadable or empty key file is an error, never a fallback to the
/// literal spec: signing with the string `file:/nope` produces a lock signed
/// by a key nobody holds, and reports success while doing it.
fn read_key_file(path: &Path, flag: &str) -> Result<String, String> {
    if path.as_os_str().is_empty() {
        return Err(format!("{flag} file:<PATH> is missing the path"));
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("{flag}: cannot read key file {}: {e}", path.display()))?;
    warn_if_shared(path, flag);
    let key = raw.trim().to_string();
    if key.is_empty() {
        return Err(format!("{flag}: key file {} is empty", path.display()));
    }
    Ok(key)
}

/// A key file other users can read is the argv leak one directory over.
///
/// ssh refuses such a key outright ("UNPROTECTED PRIVATE KEY FILE"); this
/// warns rather than refuses, because a fleet that already signs from a
/// shared file needs a release to fix its modes, not a broken pipeline.
/// (E13 quorum, agy lane.)
#[cfg(unix)]
fn warn_if_shared(path: &Path, flag: &str) {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        eprintln!(
            "warning: {flag}: key file {} is mode {mode:04o} — readable by other users on this host. \
             chmod 600 it.",
            path.display()
        );
    }
}

#[cfg(not(unix))]
fn warn_if_shared(_path: &Path, _flag: &str) {}

/// Read key material from an environment variable.
fn read_key_env(var: &str, flag: &str) -> Result<String, String> {
    if var.is_empty() {
        return Err(format!("{flag} env:<VAR> is missing the variable name"));
    }
    let raw =
        std::env::var(var).map_err(|_| format!("{flag}: environment variable {var} is not set"))?;
    let key = raw.trim().to_string();
    if key.is_empty() {
        return Err(format!("{flag}: environment variable {var} is empty"));
    }
    Ok(key)
}

/// Warn that key material was passed on `argv`, without echoing it.
fn warn_inline(flag: &str) {
    eprintln!(
        "warning: {flag} was given key material directly on the command line.\n\
         warning:   Every user on this host can read it out of `ps`.\n\
         warning:   Use {flag} file:<PATH> or {flag} env:<VAR> instead.\n\
         warning:   Inline key material is deprecated and is REMOVED in forjar {v}.",
        flag = flag,
        v = INLINE_KEY_REMOVAL_VERSION
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_recognises_the_indirect_forms() {
        assert_eq!(
            classify("file:/etc/forjar.key"),
            KeySource::File(PathBuf::from("/etc/forjar.key"))
        );
        assert_eq!(
            classify("env:FORJAR_KEY"),
            KeySource::Env("FORJAR_KEY".into())
        );
        assert_eq!(classify("hunter2"), KeySource::Inline);
        // A key that merely mentions a path is still a literal.
        assert_eq!(classify("/etc/forjar.key"), KeySource::Inline);
    }

    #[test]
    fn file_form_yields_the_contents_not_the_spec() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("k");
        std::fs::write(&p, "  top-secret\n").unwrap();
        let got = resolve(&format!("file:{}", p.display()), "--key").unwrap();
        assert_eq!(got, "top-secret");
    }

    #[test]
    fn missing_key_file_is_an_error_naming_the_flag() {
        let d = tempfile::tempdir().unwrap();
        let spec = format!("file:{}", d.path().join("absent").display());
        let err = resolve(&spec, "--old-key").unwrap_err();
        assert!(err.contains("--old-key"), "{err}");
        assert!(err.contains("cannot read key file"), "{err}");
    }

    #[test]
    fn empty_key_file_is_an_error() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("k");
        std::fs::write(&p, "\n \n").unwrap();
        let err = resolve(&format!("file:{}", p.display()), "--key").unwrap_err();
        assert!(err.contains("is empty"), "{err}");
    }

    #[test]
    fn file_form_without_a_path_is_an_error() {
        let err = resolve("file:", "--key").unwrap_err();
        assert!(err.contains("missing the path"), "{err}");
    }

    #[test]
    fn env_form_without_a_name_is_an_error() {
        let err = resolve("env:", "--key").unwrap_err();
        assert!(err.contains("missing the variable name"), "{err}");
    }

    #[test]
    fn unset_env_var_is_an_error_that_does_not_fall_back() {
        let err = resolve("env:FORJAR_KEY_SOURCE_UNSET_XYZ", "--key").unwrap_err();
        assert!(err.contains("is not set"), "{err}");
    }

    #[test]
    fn inline_form_is_returned_verbatim() {
        assert_eq!(resolve("hunter2", "--key").unwrap(), "hunter2");
    }

    #[test]
    fn resolve_opt_passes_none_through() {
        assert_eq!(resolve_opt(None, "--key").unwrap(), None);
        assert_eq!(
            resolve_opt(Some("hunter2"), "--key").unwrap(),
            Some("hunter2".to_string())
        );
    }

    #[test]
    fn help_documents_both_indirect_forms_and_the_removal() {
        assert!(KEY_ARG_HELP.contains("file:<PATH>"));
        assert!(KEY_ARG_HELP.contains("env:<VAR>"));
        assert!(KEY_ARG_HELP.contains(INLINE_KEY_REMOVAL_VERSION));
        assert!(!KEY_ARG_HELP.contains("path to key file or inline"));
    }
}
