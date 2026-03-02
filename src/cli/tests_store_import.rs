//! Tests for `forjar store-import` CLI commands.

#[cfg(test)]
mod tests {
    use crate::cli::store_import::*;

    // ── cmd_store_import ───────────────────────────────────────────

    #[test]
    fn import_apt_text_output() {
        let r = cmd_store_import("apt", "nginx", Some("1.24.0"), "/tmp/store".as_ref(), false);
        assert!(r.is_ok());
    }

    #[test]
    fn import_apt_json_output() {
        let r = cmd_store_import("apt", "nginx", Some("1.24.0"), "/tmp/store".as_ref(), true);
        assert!(r.is_ok());
    }

    #[test]
    fn import_cargo_no_version() {
        let r = cmd_store_import("cargo", "ripgrep", None, "/tmp/store".as_ref(), false);
        assert!(r.is_ok());
    }

    #[test]
    fn import_docker_json() {
        let r = cmd_store_import("docker", "ubuntu:24.04", None, "/tmp/store".as_ref(), true);
        assert!(r.is_ok());
    }

    #[test]
    fn import_nix_text() {
        let r = cmd_store_import("nix", "nixpkgs#ripgrep", None, "/tmp/store".as_ref(), false);
        assert!(r.is_ok());
    }

    #[test]
    fn import_uv_alias_pip() {
        // "pip" should resolve to Uv provider
        let r = cmd_store_import("pip", "requests", Some("2.31.0"), "/tmp/store".as_ref(), false);
        assert!(r.is_ok());
    }

    #[test]
    fn import_tofu_alias_opentofu() {
        let r = cmd_store_import("opentofu", "./infra/", None, "/tmp/store".as_ref(), false);
        assert!(r.is_ok());
    }

    #[test]
    fn import_terraform_alias_tf() {
        let r = cmd_store_import("tf", "./infra/", None, "/tmp/store".as_ref(), false);
        assert!(r.is_ok());
    }

    #[test]
    fn import_apr_text() {
        let r = cmd_store_import("apr", "meta-llama/Llama-3", None, "/tmp/store".as_ref(), false);
        assert!(r.is_ok());
    }

    #[test]
    fn import_unknown_provider_fails() {
        let r = cmd_store_import("homebrew", "nginx", None, "/tmp/store".as_ref(), false);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("unknown provider"));
    }

    // ── cmd_import_providers ───────────────────────────────────────

    #[test]
    fn list_providers_text() {
        let r = cmd_import_providers(false);
        assert!(r.is_ok());
    }

    #[test]
    fn list_providers_json() {
        let r = cmd_import_providers(true);
        assert!(r.is_ok());
    }

    // ── provider parsing edge cases ────────────────────────────────

    #[test]
    fn import_case_insensitive_provider() {
        // "APT" should work same as "apt"
        let r = cmd_store_import("APT", "nginx", None, "/tmp/store".as_ref(), false);
        assert!(r.is_ok());
    }

    #[test]
    fn import_mixed_case_provider() {
        let r = cmd_store_import("Docker", "ubuntu:24.04", None, "/tmp/store".as_ref(), true);
        assert!(r.is_ok());
    }
}
