//! Tests: Coverage for run_check_blocks in apply_output.rs — pass, fail,
//! custom exit codes, unknown machine, and transport errors (PMAT-088).

use super::apply_output::run_check_blocks;
use crate::core::types;

fn config_with_checks(checks_yaml: &str) -> types::ForjarConfig {
    let yaml = format!(
        "version: \"1.0\"\nname: checks\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {{}}\nchecks:\n{checks_yaml}"
    );
    serde_yaml_ng::from_str(&yaml).expect("valid checks config")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_all_pass_quiet() {
        let config = config_with_checks("  ok:\n    machine: m\n    command: \"true\"\n");
        run_check_blocks(&config, false);
    }

    #[test]
    fn checks_all_pass_verbose_with_description() {
        let config = config_with_checks(
            "  ok:\n    machine: m\n    command: \"true\"\n    description: health endpoint up\n",
        );
        run_check_blocks(&config, true);
    }

    #[test]
    fn checks_failure_exit_mismatch() {
        // `false` exits 1 but 0 is expected → FAIL branch + summary warning.
        let config = config_with_checks("  bad:\n    machine: m\n    command: \"false\"\n");
        run_check_blocks(&config, false);
    }

    #[test]
    fn checks_custom_expect_exit_passes() {
        // `false` exits 1 and 1 is expected → PASS with non-default exit.
        let config = config_with_checks(
            "  inverse:\n    machine: m\n    command: \"false\"\n    expect_exit: 1\n",
        );
        run_check_blocks(&config, true);
    }

    #[test]
    fn checks_unknown_machine_warns() {
        let config = config_with_checks("  ghost:\n    machine: nope\n    command: \"true\"\n");
        run_check_blocks(&config, false);
    }

    #[test]
    fn checks_transport_error_branch() {
        // `rm -rf /` is rejected by I8 bashrs validation before execution,
        // so exec_script returns Err → transport-error FAIL branch.
        let config = config_with_checks("  danger:\n    machine: m\n    command: \"rm -rf /\"\n");
        run_check_blocks(&config, false);
    }

    #[test]
    fn checks_mixed_pass_and_fail_verbose() {
        let config = config_with_checks(
            "  ok:\n    machine: m\n    command: \"true\"\n  bad:\n    machine: m\n    command: \"false\"\n    description: should warn\n",
        );
        run_check_blocks(&config, true);
    }
}
