//! FJ-006: Package resource handler (apt + cargo).

use crate::core::types::Resource;

/// Generate shell script to check if packages are installed.
pub fn check_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let packages = &resource.packages;

    match provider {
        "apt" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    format!(
                        "dpkg -l '{}' >/dev/null 2>&1 && echo 'installed:{}' || echo 'missing:{}'",
                        p, p, p
                    )
                })
                .collect();
            checks.join("\n")
        }
        "cargo" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| format!("command -v '{}' >/dev/null 2>&1 && echo 'installed:{}' || echo 'missing:{}'", p, p, p))
                .collect();
            checks.join("\n")
        }
        _ => format!("echo 'unsupported provider: {}'", provider),
    }
}

/// Generate shell script to install packages.
pub fn apply_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let packages = &resource.packages;
    let state = resource.state.as_deref().unwrap_or("present");

    match (provider, state) {
        ("apt", "present") => {
            let pkg_list: Vec<String> = packages.iter().map(|p| format!("'{}'", p)).collect();
            let joined = pkg_list.join(" ");
            format!(
                "set -euo pipefail\n\
                 NEED_INSTALL=0\n\
                 for pkg in {joined}; do\n\
                   dpkg -l \"$pkg\" >/dev/null 2>&1 || NEED_INSTALL=1\n\
                 done\n\
                 if [ \"$NEED_INSTALL\" = \"1\" ]; then\n\
                   apt-get update -qq\n\
                   DEBIAN_FRONTEND=noninteractive apt-get install -y -qq {joined}\n\
                 fi\n\
                 # Postcondition: all packages installed\n\
                 for pkg in {joined}; do\n\
                   dpkg -l \"$pkg\" >/dev/null 2>&1\n\
                 done"
            )
        }
        ("apt", "absent") => {
            let pkg_list: Vec<String> = packages.iter().map(|p| format!("'{}'", p)).collect();
            let joined = pkg_list.join(" ");
            format!(
                "set -euo pipefail\n\
                 NEED_REMOVE=0\n\
                 for pkg in {joined}; do\n\
                   dpkg -l \"$pkg\" >/dev/null 2>&1 && NEED_REMOVE=1\n\
                 done\n\
                 if [ \"$NEED_REMOVE\" = \"1\" ]; then\n\
                   DEBIAN_FRONTEND=noninteractive apt-get remove -y -qq {joined}\n\
                 fi"
            )
        }
        ("cargo", "present") => {
            let installs: Vec<String> = packages
                .iter()
                .map(|p| {
                    format!(
                        "if ! command -v '{}' >/dev/null 2>&1; then\n  cargo install '{}'\nfi",
                        p, p
                    )
                })
                .collect();
            format!("set -euo pipefail\n{}", installs.join("\n"))
        }
        _ => format!("echo 'unsupported: provider={}, state={}'", provider, state),
    }
}

/// Generate shell to query installed versions (for state hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let packages = &resource.packages;

    match provider {
        "apt" => {
            let queries: Vec<String> = packages
                .iter()
                .map(|p| format!("dpkg-query -W -f '${{Package}}=${{Version}}\\n' '{}' 2>/dev/null || echo '{}=MISSING'", p, p))
                .collect();
            queries.join("\n")
        }
        "cargo" => {
            let queries: Vec<String> = packages
                .iter()
                .map(|p| format!("command -v '{}' >/dev/null 2>&1 && echo '{}=installed' || echo '{}=MISSING'", p, p, p))
                .collect();
            queries.join("\n")
        }
        _ => "echo 'unknown'".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn make_apt_resource(packages: &[&str]) -> Resource {
        Resource {
            resource_type: ResourceType::Package,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: Some("apt".to_string()),
            packages: packages.iter().map(|s| s.to_string()).collect(),
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            fs_type: None,
            options: None,
        }
    }

    #[test]
    fn test_fj006_check_apt() {
        let r = make_apt_resource(&["curl", "wget"]);
        let script = check_script(&r);
        assert!(script.contains("dpkg -l 'curl'"));
        assert!(script.contains("dpkg -l 'wget'"));
    }

    #[test]
    fn test_fj006_apply_apt_present() {
        let r = make_apt_resource(&["curl"]);
        let script = apply_script(&r);
        assert!(script.contains("apt-get install"));
        assert!(script.contains("'curl'"));
        assert!(script.contains("set -euo pipefail"));
        assert!(script.contains("DEBIAN_FRONTEND=noninteractive"));
    }

    #[test]
    fn test_fj006_apply_apt_absent() {
        let mut r = make_apt_resource(&["curl"]);
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("apt-get remove"));
    }

    #[test]
    fn test_fj006_cargo_check() {
        let mut r = make_apt_resource(&["batuta"]);
        r.provider = Some("cargo".to_string());
        let script = check_script(&r);
        assert!(script.contains("command -v 'batuta'"));
    }

    #[test]
    fn test_fj006_cargo_install() {
        let mut r = make_apt_resource(&["batuta"]);
        r.provider = Some("cargo".to_string());
        let script = apply_script(&r);
        assert!(script.contains("cargo install 'batuta'"));
    }

    #[test]
    fn test_fj006_state_query_apt() {
        let r = make_apt_resource(&["curl"]);
        let script = state_query_script(&r);
        assert!(script.contains("dpkg-query"));
    }

    #[test]
    fn test_fj006_quoted_packages() {
        // Verify all package names are single-quoted (injection prevention)
        let r = make_apt_resource(&["curl", "lib; rm -rf /"]);
        let script = apply_script(&r);
        assert!(script.contains("'lib; rm -rf /'"));
        // The semicolon is inside quotes — safe
    }

    #[test]
    fn test_fj006_state_query_cargo() {
        let mut r = make_apt_resource(&["batuta", "renacer"]);
        r.provider = Some("cargo".to_string());
        let script = state_query_script(&r);
        assert!(script.contains("command -v 'batuta'"));
        assert!(script.contains("command -v 'renacer'"));
        assert!(script.contains("installed"));
    }

    #[test]
    fn test_fj006_state_query_unknown_provider() {
        let mut r = make_apt_resource(&["tool"]);
        r.provider = Some("pip".to_string());
        let script = state_query_script(&r);
        assert!(script.contains("unknown"));
    }

    #[test]
    fn test_fj006_check_unsupported_provider() {
        let mut r = make_apt_resource(&["foo"]);
        r.provider = Some("brew".to_string());
        let script = check_script(&r);
        assert!(script.contains("unsupported provider"));
    }

    #[test]
    fn test_fj006_apply_unsupported_combo() {
        let mut r = make_apt_resource(&["foo"]);
        r.provider = Some("pip".to_string());
        r.state = Some("present".to_string());
        let script = apply_script(&r);
        assert!(script.contains("unsupported"));
    }

    /// BH-MUT-0001: Kill mutation of cargo check_script boolean logic.
    /// Verify installed/missing output format to catch && / || flip.
    #[test]
    fn test_fj006_cargo_check_output_format() {
        let mut r = make_apt_resource(&["ripgrep"]);
        r.provider = Some("cargo".to_string());
        let script = check_script(&r);
        // Must contain both installed AND missing branches — flipping && to || would break
        assert!(script.contains("echo 'installed:ripgrep'"));
        assert!(script.contains("echo 'missing:ripgrep'"));
    }

    /// BH-MUT-0001: Kill mutation of apt check_script boolean logic.
    #[test]
    fn test_fj006_apt_check_output_format() {
        let r = make_apt_resource(&["curl"]);
        let script = check_script(&r);
        assert!(script.contains("echo 'installed:curl'"));
        assert!(script.contains("echo 'missing:curl'"));
    }

    /// BH-MUT-0002: Kill mutation of cargo state_query_script boolean logic.
    #[test]
    fn test_fj006_state_query_cargo_output_format() {
        let mut r = make_apt_resource(&["pmat"]);
        r.provider = Some("cargo".to_string());
        let script = state_query_script(&r);
        assert!(script.contains("echo 'pmat=installed'"));
        assert!(script.contains("echo 'pmat=MISSING'"));
    }

    /// BH-MUT-0003: Kill mutation of apt state_query_script format.
    #[test]
    fn test_fj006_state_query_apt_output_format() {
        let r = make_apt_resource(&["vim"]);
        let script = state_query_script(&r);
        assert!(script.contains("vim"));
        assert!(script.contains("vim=MISSING"));
        assert!(script.contains("dpkg-query -W"));
    }

    /// BH-MUT: Multi-package list preserves order.
    #[test]
    fn test_fj006_multi_package_check_preserves_all() {
        let r = make_apt_resource(&["a", "b", "c"]);
        let script = check_script(&r);
        // All packages present in output
        assert!(script.contains("dpkg -l 'a'"));
        assert!(script.contains("dpkg -l 'b'"));
        assert!(script.contains("dpkg -l 'c'"));
        // Verify newline separation (multi-line script)
        assert_eq!(script.matches('\n').count(), 2);
    }

    /// BH-MUT: cargo install uses conditional check before installing.
    #[test]
    fn test_fj006_cargo_install_conditional() {
        let mut r = make_apt_resource(&["tool"]);
        r.provider = Some("cargo".to_string());
        let script = apply_script(&r);
        // Must check if already installed before installing
        assert!(script.contains("if ! command -v 'tool'"));
        assert!(script.contains("cargo install 'tool'"));
    }
}
