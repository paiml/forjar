//! #1187: surface cloud_instance volume-tag markers from a passing check.

use super::check::{print_check_failure, print_unmeasured};

/// The only stdout lines of an exit-0 check that are shown: a line that starts with one of
/// these (what `cloud_instance` writes when a live instance's volumes miss declared tags).
const MARKERS: [&str; 2] = ["volume-tags-drift:", "volume-tags-unreadable:"];

/// Marker lines of a passing check's stdout, one per line; everything else is dropped.
pub(super) fn pass_markers(stdout: &str) -> String {
    let hit = |l: &&str| MARKERS.iter().any(|m| l.starts_with(m));
    stdout.lines().filter(hit).collect::<Vec<_>>().join("\n")
}

/// Print one check outcome in text mode: ok (plus any marker warnings), unmeasured or FAIL.
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
    } else if passed {
        println!("  ok {resource_id} ({machine_name})");
        for line in detail.lines() {
            println!("       warn: {line}");
        }
    } else {
        let msg = match exit_code {
            Some(code) => format!("exit {code}"),
            None => detail.to_string(),
        };
        print_check_failure(resource_id, machine_name, &msg);
        if exit_code.is_some() {
            for line in detail.lines().filter(|l| !l.is_empty()) {
                println!("       {line}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::check::{localhost_machine, run_single_check};

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
}
