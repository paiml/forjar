//! Package `check_script` generation.
//!
//! Split out of `package.rs` to keep both files under the 500-line limit. The
//! four providers each assert one line per package so a partially-installed
//! set names the packages that are actually missing (FJ-2720).

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::package::parse_cargo_features;
use crate::resources::verdict;

/// Generate shell script to check if packages are installed.
pub fn check_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let packages = &resource.packages;

    match provider {
        "apt" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    let q = sh_squote(p);
                    verdict::assert_that(
                        &format!("dpkg -l {q} 2>/dev/null | grep -q '^ii '"),
                        &format!("installed:{p}"),
                        &format!("missing:{p}"),
                    )
                })
                .collect();
            verdict::check_script_from(&checks)
        }
        "cargo" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    let (crate_name, _) = parse_cargo_features(p);
                    let q = sh_squote(crate_name);
                    verdict::assert_that(
                        &format!("command -v {q} >/dev/null 2>&1"),
                        &format!("installed:{crate_name}"),
                        &format!("missing:{crate_name}"),
                    )
                })
                .collect();
            verdict::check_script_from(&checks)
        }
        "uv" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    verdict::assert_that(
                        &format!(
                            "uv tool list 2>/dev/null | grep -q {}",
                            sh_squote(&format!("^{p}"))
                        ),
                        &format!("installed:{p}"),
                        &format!("missing:{p}"),
                    )
                })
                .collect();
            verdict::check_script_from(&checks)
        }
        "brew" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    let q = sh_squote(p);
                    verdict::assert_that(
                        &format!("brew list {q} >/dev/null 2>&1"),
                        &format!("installed:{p}"),
                        &format!("missing:{p}"),
                    )
                })
                .collect();
            verdict::check_script_from(&checks)
        }
        other => verdict::check_script_from(&[verdict::always_diverged(&format!(
            "unsupported provider: {other}"
        ))]),
    }
}
