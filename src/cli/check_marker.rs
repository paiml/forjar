//! #1187: surface cloud_instance volume-tag markers from a passing check.

use super::check::print_unmeasured;

/// The only stdout lines of an exit-0 check that are shown: a line that starts with one of
/// these (what `cloud_instance` writes when a live instance's volumes miss declared tags).
const MARKERS: [&str; 2] = ["volume-tags-drift:", "volume-tags-unreadable:"];

/// Whether a stdout line starts with one of the allowlisted marker prefixes.
fn is_marker(line: &str) -> bool {
    MARKERS.iter().any(|m| line.starts_with(m))
}

/// Marker lines of a passing check's stdout, one per line; everything else is dropped.
pub(super) fn pass_markers(stdout: &str) -> String {
    stdout
        .lines()
        .filter(|l| is_marker(l))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Text-mode lines of one check outcome: `ok` (plus a `warn:` line per allowlisted marker line,
/// only when `status` is "pass"; a skip prints `ok` alone) or FAIL. Unmeasured is printed by
/// `print_check_outcome`.
pub(super) fn check_outcome_lines(
    resource_id: &str,
    machine_name: &str,
    status: &str,
    passed: bool,
    exit_code: Option<i32>,
    detail: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    if passed {
        lines.push(format!("  ok {resource_id} ({machine_name})"));
        if status == "pass" {
            for line in detail.lines().filter(|l| is_marker(l)) {
                lines.push(format!("       warn: {line}"));
            }
        }
    } else {
        let msg = match exit_code {
            Some(code) => format!("exit {code}"),
            None => detail.to_string(),
        };
        lines.push(format!("  FAIL {resource_id} ({machine_name}) — {msg}"));
        if exit_code.is_some() {
            for line in detail.lines().filter(|l| !l.is_empty()) {
                lines.push(format!("       {line}"));
            }
        }
    }
    lines
}

/// Print one check outcome in text mode: the lines of `check_outcome_lines`, or UNMEASURED.
pub(super) fn print_check_outcome(
    resource_id: &str,
    machine_name: &str,
    status: &str,
    passed: bool,
    exit_code: Option<i32>,
    detail: &str,
) {
    if status == "unmeasured" {
        print_unmeasured(resource_id, machine_name, detail);
        return;
    }
    let lines = check_outcome_lines(resource_id, machine_name, status, passed, exit_code, detail);
    for line in lines {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::super::check::{localhost_machine, run_single_check};
    use super::check_outcome_lines;

    fn run(script: &str) -> (String, String) {
        let (r, passed) = run_single_check(&localhost_machine(), script, "web", "m", true);
        assert!(passed, "an exit-0 check stays a pass: {}", r.detail);
        (r.status, r.detail)
    }

    #[test]
    fn passing_check_surfaces_volume_tag_drift_and_unreadable_markers() {
        let (st, d) = run("echo volume-tags-drift:k1; exit 0");
        assert_eq!((st.as_str(), d.as_str()), ("pass", "volume-tags-drift:k1"));
        let (_, d) = run("echo volume-tags-unreadable:k1; exit 0");
        assert_eq!(d, "volume-tags-unreadable:k1");
    }

    #[test]
    fn passing_check_without_a_marker_prints_nothing_new() {
        for script in [
            "exit 0",
            "echo volume-tags-match:k1; exit 0",
            "echo unrelated output; exit 0",
            "echo 'x volume-tags-drift:k1'; exit 0",
        ] {
            let (st, d) = run(script);
            assert_eq!((st.as_str(), d.as_str()), ("pass", ""), "{script}");
        }
    }

    fn lines_for(script: &str) -> Vec<String> {
        let (r, passed) = run_single_check(&localhost_machine(), script, "web", "m", true);
        let (st, code, d) = (r.status.as_str(), r.exit_code, r.detail.as_str());
        check_outcome_lines("web", "m", st, passed, code, d)
    }

    #[test]
    fn skip_prints_ok_alone_without_warn_lines() {
        let lines = lines_for("echo 'FORJAR_SKIP: no systemd'; echo volume-tags-drift:k1; exit 2");
        assert_eq!(lines, vec!["  ok web (m)".to_string()]);
    }

    #[test]
    fn pass_with_marker_prints_ok_then_warn_in_text_mode() {
        let lines = lines_for("echo volume-tags-drift:k1; exit 0");
        let want = ["  ok web (m)", "       warn: volume-tags-drift:k1"];
        assert_eq!(lines, want);
    }

    #[test]
    fn pass_prints_only_allowlisted_marker_lines() {
        let detail = "FORJAR_SKIP: x\nvolume-tags-unreadable:k1\nnoise";
        let lines = check_outcome_lines("web", "m", "pass", true, Some(0), detail);
        let want = ["  ok web (m)", "       warn: volume-tags-unreadable:k1"];
        assert_eq!(lines, want);
    }
}
