//! FJ-006: Package resource handler (apt + cargo + uv).

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
        "uv" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| format!("uv tool list 2>/dev/null | grep -q '^{}' && echo 'installed:{}' || echo 'missing:{}'", p, p, p))
                .collect();
            checks.join("\n")
        }
        other => format!("echo 'unsupported provider: {}'", other),
    }
}

/// Generate shell script to install packages.
pub fn apply_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let packages = &resource.packages;
    let state = resource.state.as_deref().unwrap_or("present");
    let version = resource.version.as_deref();

    match (provider, state) {
        ("apt", "present") => {
            // For apt: package=version pins to exact version
            let pkg_list: Vec<String> = packages
                .iter()
                .map(|p| match version {
                    Some(v) => format!("'{}={}'", p, v),
                    None => format!("'{}'", p),
                })
                .collect();
            let check_list: Vec<String> = packages.iter().map(|p| format!("'{}'", p)).collect();
            let joined = pkg_list.join(" ");
            let check_joined = check_list.join(" ");
            format!(
                "set -euo pipefail\n\
                 SUDO=\"\"\n\
                 [ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"\n\
                 NEED_INSTALL=0\n\
                 for pkg in {check_joined}; do\n\
                   dpkg -l \"$pkg\" >/dev/null 2>&1 || NEED_INSTALL=1\n\
                 done\n\
                 if [ \"$NEED_INSTALL\" = \"1\" ]; then\n\
                   $SUDO apt-get update -qq\n\
                   DEBIAN_FRONTEND=noninteractive $SUDO apt-get install -y -qq {joined}\n\
                 fi\n\
                 # Postcondition: all packages installed\n\
                 for pkg in {check_joined}; do\n\
                   dpkg -l \"$pkg\" >/dev/null 2>&1\n\
                 done"
            )
        }
        ("apt", "absent") => {
            let pkg_list: Vec<String> = packages.iter().map(|p| format!("'{}'", p)).collect();
            let joined = pkg_list.join(" ");
            format!(
                "set -euo pipefail\n\
                 SUDO=\"\"\n\
                 [ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"\n\
                 NEED_REMOVE=0\n\
                 for pkg in {joined}; do\n\
                   dpkg -l \"$pkg\" >/dev/null 2>&1 && NEED_REMOVE=1\n\
                 done\n\
                 if [ \"$NEED_REMOVE\" = \"1\" ]; then\n\
                   DEBIAN_FRONTEND=noninteractive $SUDO apt-get remove -y -qq {joined}\n\
                 fi"
            )
        }
        ("cargo", "present") => {
            // For cargo: cargo install package@version
            let installs: Vec<String> = packages
                .iter()
                .map(|p| match version {
                    Some(v) => format!("cargo install --force '{}@{}'", p, v),
                    None => format!("cargo install --force '{}'", p),
                })
                .collect();
            format!("set -euo pipefail\n{}", installs.join("\n"))
        }
        ("uv", "present") => {
            // For uv: uv tool install package==version
            let installs: Vec<String> = packages
                .iter()
                .map(|p| match version {
                    Some(v) => format!("uv tool install --force '{}=={}'", p, v),
                    None => format!("uv tool install --force '{}'", p),
                })
                .collect();
            format!("set -euo pipefail\n{}", installs.join("\n"))
        }
        ("uv", "absent") => {
            let removals: Vec<String> = packages
                .iter()
                .map(|p| format!("uv tool uninstall '{}' 2>/dev/null || true", p))
                .collect();
            format!("set -euo pipefail\n{}", removals.join("\n"))
        }
        (other_provider, other_state) => format!(
            "echo 'unsupported: provider={}, state={}'",
            other_provider, other_state
        ),
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
        "uv" => {
            let queries: Vec<String> = packages
                .iter()
                .map(|p| format!("uv tool list 2>/dev/null | grep -q '^{}' && echo '{}=installed' || echo '{}=MISSING'", p, p, p))
                .collect();
            queries.join("\n")
        }
        other => format!("echo 'unsupported provider: {}'", other),
    }
}
