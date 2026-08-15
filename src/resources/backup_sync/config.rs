//! FJ-037: forjar-managed `rclone.conf`.
//!
//! # Why forjar owns this file
//!
//! An rclone remote that is not configured does not fail — rclone falls back to
//! treating `name:path` as a *local* path. That is the same class of failure as
//! the predecessor on lambda-labs, which synced `/mnt/nvme-raid0/videos` to
//! `/videos` (a symlink back to itself) and reported success hourly for months.
//! Leaving the remote definition as a manual step reintroduces exactly that
//! risk, so the definition is declared state like anything else.
//!
//! # The split, and why it is a split
//!
//! `rclone.conf` mixes two things with very different handling rules:
//!
//!   * **Configuration** — backend, scope, folder id, tuning. Belongs in the
//!     repo, reviewable, diffable, drift-checked.
//!   * **The OAuth refresh token** — a bearer credential. Must never be
//!     committed in cleartext.
//!
//! So `backup_remote_config` is written verbatim from the declaration, and
//! `backup_token` is expected to arrive as `{{secrets.NAME}}`, resolved through
//! whichever provider the config declares (`sops`/age keeps the ciphertext in
//! the repo). The generated file is written 0600, and the token is redacted
//! everywhere it could otherwise surface.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;

/// Path of the managed rclone config for a given home directory.
pub(super) fn conf_path(home: &str) -> String {
    format!("{}/.config/rclone/rclone.conf", home.trim_end_matches('/'))
}

/// A token value that is still an unresolved template never reached the
/// secrets provider — writing it would produce a config whose password is the
/// literal string `{{secrets.foo}}`.
pub(super) fn is_unresolved(token: &str) -> bool {
    token.contains("{{") && token.contains("}}")
}

/// Render the `[remote]` stanza. Keys are emitted sorted so the file — and
/// therefore its hash, and therefore drift — is stable across runs.
pub(super) fn stanza(resource: &Resource, remote_name: &str, token: Option<&str>) -> String {
    let backend = resource.backup.remote_type.as_deref().unwrap_or("drive");
    let mut keys: Vec<(&String, &String)> = resource.backup.remote_config.iter().collect();
    keys.sort_by(|a, b| a.0.cmp(b.0));

    let mut out = format!("[{remote_name}]\ntype = {backend}\n");
    for (k, v) in keys {
        // `token` is owned by the secret path; a config key of that name would
        // silently shadow it.
        if k == "token" {
            continue;
        }
        out.push_str(&format!("{k} = {v}\n"));
    }
    if let Some(t) = token {
        out.push_str(&format!("token = {t}\n"));
    }
    out
}

/// Shell that installs the config atomically at 0600.
///
/// Written via a quoted heredoc so nothing in the token is expanded by the
/// shell, to a temp file in the same directory (atomic rename), and with the
/// mode set *before* the content lands so the token is never briefly readable.
pub(super) fn install(home: &str, body: &str) -> String {
    let path = conf_path(home);
    let path_q = sh_squote(&path);
    let dir_q = sh_squote(&format!("{}/.config/rclone", home.trim_end_matches('/')));
    format!(
        r#"mkdir -p {dir_q}
chmod 0700 {dir_q}
BS_CONF_TMP={path_q}.tmp
# umask before creation: the token must never exist world-readable, not even
# for the instant between creat() and chmod().
( umask 077; cat > "$BS_CONF_TMP" <<'FORJAR_RCLONE_CONF'
{body}
FORJAR_RCLONE_CONF
)
chmod 0600 "$BS_CONF_TMP"
mv -f "$BS_CONF_TMP" {path_q}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{BackupSpec, ResourceType};
    use std::collections::HashMap;

    fn res() -> Resource {
        let mut cfg = HashMap::new();
        cfg.insert("scope".to_string(), "drive.file".to_string());
        cfg.insert("root_folder_id".to_string(), "ABC123".to_string());
        Resource {
            resource_type: ResourceType::BackupSync,
            backup: BackupSpec {
                remote_config: cfg,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn stanza_declares_the_backend_and_config_keys() {
        let s = stanza(&res(), "gdrive", None);
        assert!(s.starts_with("[gdrive]\ntype = drive\n"));
        assert!(s.contains("scope = drive.file"));
        assert!(s.contains("root_folder_id = ABC123"));
    }

    #[test]
    fn backend_is_overridable() {
        let r = Resource {
            backup: BackupSpec {
                remote_type: Some("s3".into()),
                ..res().backup
            },
            ..res()
        };
        assert!(stanza(&r, "x", None).contains("type = s3"));
    }

    #[test]
    fn keys_are_sorted_so_the_hash_is_stable() {
        // HashMap iteration order varies per process; an unsorted stanza would
        // make the config hash — and therefore drift — flap at random.
        let a = stanza(&res(), "gdrive", None);
        for _ in 0..8 {
            assert_eq!(stanza(&res(), "gdrive", None), a);
        }
        let scope = a.find("scope").unwrap();
        let root = a.find("root_folder_id").unwrap();
        assert!(root < scope, "keys must be sorted");
    }

    #[test]
    fn a_config_key_named_token_cannot_shadow_the_secret() {
        let mut r = res();
        r.backup
            .remote_config
            .insert("token".into(), "NOT-THE-REAL-TOKEN".into());
        let s = stanza(&r, "gdrive", Some("REAL"));
        assert!(!s.contains("NOT-THE-REAL-TOKEN"));
        assert_eq!(s.matches("token = ").count(), 1);
        assert!(s.contains("token = REAL"));
    }

    #[test]
    fn token_is_omitted_when_absent() {
        assert!(!stanza(&res(), "gdrive", None).contains("token ="));
    }

    #[test]
    fn detects_an_unresolved_secret_template() {
        // Writing this literally yields a config whose credential is the string
        // "{{secrets.rclone_token}}" — rclone would fail in a way that looks
        // like an auth problem rather than a config-resolution bug.
        assert!(is_unresolved("{{secrets.rclone_token}}"));
        assert!(!is_unresolved("ya29.a0Af..."));
    }

    #[test]
    fn install_is_atomic_and_never_world_readable() {
        let s = install("/home/noah", "[gdrive]\ntype = drive\n");
        assert!(
            s.contains("umask 077"),
            "token must not exist world-readable"
        );
        assert!(s.contains("chmod 0600"));
        assert!(s.contains("chmod 0700"));
        assert!(s.contains("mv -f"), "install must be atomic");
    }

    #[test]
    fn install_uses_a_quoted_heredoc() {
        // An unquoted heredoc would let `$` or backticks in a token be expanded
        // by the shell, silently corrupting the credential.
        assert!(install("/home/noah", "x").contains("<<'FORJAR_RCLONE_CONF'"));
    }

    #[test]
    fn conf_path_is_the_standard_location() {
        assert_eq!(
            conf_path("/home/noah"),
            "/home/noah/.config/rclone/rclone.conf"
        );
        assert_eq!(
            conf_path("/home/noah/"),
            "/home/noah/.config/rclone/rclone.conf"
        );
    }
}
