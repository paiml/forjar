//! FJ-33: Build resource handler — cross-compile on one machine, deploy to another.
//!
//! The build resource generates scripts that run on the **deploy machine**.
//! The apply script SSHes to the build machine to compile, then SCPs
//! the artifact back to the deploy target.
//!
//! YAML example:
//! ```yaml
//! apr-binary:
//!   type: build
//!   machine: jetson            # deploy target
//!   build_machine: intel       # where compilation runs
//!   command: "cargo build --release --target aarch64-unknown-linux-gnu -p apr-cli"
//!   working_dir: ~/src/aprender
//!   source: /tmp/cross/release/apr   # artifact path on build machine
//!   target: ~/.cargo/bin/apr         # deploy path on target machine
//!   completion_check: "apr --version"
//! ```

use crate::core::types::Resource;

/// Check if the deployed artifact exists and is executable.
pub fn check_script(resource: &Resource) -> String {
    let deploy_path = resource.target.as_deref().unwrap_or("/dev/null");
    let mut script =
        format!("test -x '{deploy_path}' && echo 'installed:build' || echo 'missing:build'");
    if let Some(ref check) = resource.completion_check {
        script = format!(
            "if {check} >/dev/null 2>&1; then echo 'installed:build'; else echo 'missing:build'; fi"
        );
    }
    script
}

/// Generate the build + transfer + deploy script.
///
/// Runs on the deploy machine. Phases:
/// 1. SSH to build_machine, run build command in working_dir
/// 2. SCP artifact from build_machine to deploy_path
/// 3. Run completion_check locally
pub fn apply_script(resource: &Resource) -> String {
    let command = resource.command.as_deref().unwrap_or("true");
    let artifact = resource.source.as_deref().unwrap_or("/dev/null");
    let deploy_path = resource.target.as_deref().unwrap_or("/tmp/build-artifact");
    let build_machine = resource.build_machine.as_deref().unwrap_or("localhost");

    let mut script = String::from("set -euo pipefail\n");

    // Phase 1: Build on the build machine
    let build_cmd = if let Some(ref dir) = resource.working_dir {
        format!("cd '{dir}' && {command}")
    } else {
        command.to_string()
    };

    // Use build_machine as a bare SSH target. If build_machine is "localhost",
    // run locally; otherwise SSH to it.
    if build_machine == "localhost" {
        script.push_str(&format!("# Phase 1: build locally\n{build_cmd}\n"));
    } else {
        script.push_str(&format!(
            "# Phase 1: build on {build_machine}\n\
             ssh -o BatchMode=yes -o ConnectTimeout=10 '{build_machine}' '{build_cmd}'\n"
        ));
    }

    // Phase 2: Transfer artifact
    if build_machine != "localhost" {
        script.push_str(&format!(
            "# Phase 2: transfer artifact\n\
             mkdir -p \"$(dirname '{deploy_path}')\"\n\
             scp -o BatchMode=yes '{build_machine}:{artifact}' '{deploy_path}'\n\
             chmod +x '{deploy_path}'\n"
        ));
    } else {
        script.push_str(&format!(
            "# Phase 2: copy artifact locally\n\
             mkdir -p \"$(dirname '{deploy_path}')\"\n\
             cp '{artifact}' '{deploy_path}'\n\
             chmod +x '{deploy_path}'\n"
        ));
    }

    // Phase 3: Completion check
    if let Some(ref check) = resource.completion_check {
        script.push_str(&format!("# Phase 3: verify\n{check}\n"));
    }

    script
}

/// State query: hash the deployed artifact for drift detection.
pub fn state_query_script(resource: &Resource) -> String {
    let deploy_path = resource.target.as_deref().unwrap_or("/dev/null");
    format!(
        "if [ -f '{deploy_path}' ]; then \
         sha256sum '{deploy_path}' | awk '{{print $1}}'; \
         else echo 'MISSING'; fi"
    )
}
