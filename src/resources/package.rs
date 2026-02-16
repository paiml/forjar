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
                .map(|p| format!("dpkg -l '{}' >/dev/null 2>&1 && echo 'installed:{}' || echo 'missing:{}'", p, p, p))
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
}
