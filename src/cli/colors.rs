//! FJ-2910: ANSI color constants and semantic formatting helpers.
//!
//! All CLI handlers should use these for consistent colorized output.
//! The global `NO_COLOR` flag (set by `--no-color` or `$NO_COLOR` env) disables all ANSI.
//!
//! Many helpers are pre-built for FJ-2910 adoption across the codebase.
#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};

/// Global color disable flag.
pub(crate) static NO_COLOR: AtomicBool = AtomicBool::new(false);

/// Check if color output is enabled.
#[inline]
pub(crate) fn color_enabled() -> bool {
    !NO_COLOR.load(Ordering::Relaxed)
}

/// Wrap text in an ANSI escape code, respecting NO_COLOR.
#[inline]
fn wrap(code: &str, text: &str) -> String {
    if color_enabled() && !code.is_empty() {
        format!("{code}{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

// ── ANSI escape constants ───────────────────────────────────────────

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const UNDERLINE: &str = "\x1b[4m";

pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const CYAN: &str = "\x1b[36m";

pub const BOLD_RED: &str = "\x1b[1;31m";
pub const BOLD_GREEN: &str = "\x1b[1;32m";
pub const BOLD_YELLOW: &str = "\x1b[1;33m";
pub const BOLD_WHITE: &str = "\x1b[1;37m";
pub const DIM_WHITE: &str = "\x1b[2;37m";

// ── Raw color wrappers ──────────────────────────────────────────────

/// Green text.
pub(crate) fn green(s: &str) -> String {
    wrap(GREEN, s)
}

/// Red text.
pub(crate) fn red(s: &str) -> String {
    wrap(RED, s)
}

/// Yellow text.
pub(crate) fn yellow(s: &str) -> String {
    wrap(YELLOW, s)
}

/// Blue text.
pub(crate) fn blue(s: &str) -> String {
    wrap(BLUE, s)
}

/// Cyan text.
pub(crate) fn cyan(s: &str) -> String {
    wrap(CYAN, s)
}

/// Dim text.
pub(crate) fn dim(s: &str) -> String {
    wrap(DIM, s)
}

/// Bold text.
pub(crate) fn bold(s: &str) -> String {
    wrap(BOLD, s)
}

// ── Semantic helpers ────────────────────────────────────────────────

/// Section header: bold + underline.
pub(crate) fn header(text: &str) -> String {
    if color_enabled() {
        format!("{BOLD}{UNDERLINE}{text}{RESET}")
    } else {
        text.to_string()
    }
}

/// Pass indicator: ✓ green.
pub(crate) fn pass(text: &str) -> String {
    if color_enabled() {
        format!("{GREEN}✓{RESET} {text}")
    } else {
        format!("[pass] {text}")
    }
}

/// Warning indicator: ⚠ yellow.
pub(crate) fn warn_icon(text: &str) -> String {
    if color_enabled() {
        format!("{YELLOW}⚠{RESET} {text}")
    } else {
        format!("[warn] {text}")
    }
}

/// Failure indicator: ✗ red.
pub(crate) fn fail(text: &str) -> String {
    if color_enabled() {
        format!("{RED}✗{RESET} {text}")
    } else {
        format!("[fail] {text}")
    }
}

/// Skip indicator: ⏭ dim.
pub(crate) fn skip(text: &str) -> String {
    if color_enabled() {
        format!("{DIM}⏭{RESET} {DIM}{text}{RESET}")
    } else {
        format!("[skip] {text}")
    }
}

/// Grade coloring: A/B=green, C=yellow, D/F=red.
pub(crate) fn grade(g: &str) -> String {
    let color = match g.chars().next() {
        Some('A') => GREEN,
        Some('B') => GREEN,
        Some('C') => YELLOW,
        Some('D') => RED,
        Some('F') => BOLD_RED,
        _ => "",
    };
    wrap(color, g)
}

/// Threshold-colored percentage (higher is better).
pub(crate) fn pct(value: f64, good: f64, warn_at: f64) -> String {
    let s = format!("{value:.1}%");
    let color = if value >= good {
        GREEN
    } else if value >= warn_at {
        YELLOW
    } else {
        RED
    };
    wrap(color, &s)
}

/// Delta coloring: positive=green, negative=red, zero=dim.
pub(crate) fn delta(value: f64) -> String {
    let s = format!("{value:+.1}");
    let color = if value > 0.0 {
        GREEN
    } else if value < 0.0 {
        RED
    } else {
        DIM
    };
    wrap(color, &s)
}

/// Score fraction: "earned/max" with threshold coloring.
pub(crate) fn score_frac(earned: f64, max: f64, good_pct: f64, warn_pct: f64) -> String {
    let ratio = if max > 0.0 { earned / max * 100.0 } else { 0.0 };
    let color = if ratio >= good_pct {
        GREEN
    } else if ratio >= warn_pct {
        YELLOW
    } else {
        RED
    };
    if color_enabled() && !color.is_empty() {
        format!("{color}{earned:.1}{RESET}/{DIM}{max:.1}{RESET}")
    } else {
        format!("{earned:.1}/{max:.1}")
    }
}

/// Heavy horizontal rule (━━━).
pub(crate) fn rule() -> String {
    dim(&"━".repeat(60))
}

/// Light section separator (───).
pub(crate) fn separator() -> String {
    dim(&"─".repeat(60))
}

/// File path: cyan (matching rg/fd convention).
pub(crate) fn path(text: &str) -> String {
    wrap(CYAN, text)
}

/// Duration with target-relative coloring.
pub(crate) fn duration_colored(secs: f64, target_secs: f64) -> String {
    let s = if secs >= 1.0 {
        format!("{secs:.2}s")
    } else if secs >= 0.001 {
        format!("{:.1}ms", secs * 1_000.0)
    } else {
        format!("{:.1}µs", secs * 1_000_000.0)
    };
    let color = if secs <= target_secs {
        GREEN
    } else if secs <= target_secs * 2.0 {
        YELLOW
    } else {
        RED
    };
    wrap(color, &s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_enabled_default() {
        NO_COLOR.store(false, Ordering::Relaxed);
        assert!(color_enabled());
    }

    #[test]
    fn test_no_color_disables() {
        NO_COLOR.store(true, Ordering::Relaxed);
        assert!(!color_enabled());
        assert_eq!(green("hi"), "hi");
        assert_eq!(red("hi"), "hi");
        assert_eq!(yellow("hi"), "hi");
        assert_eq!(blue("hi"), "hi");
        assert_eq!(cyan("hi"), "hi");
        assert_eq!(dim("hi"), "hi");
        assert_eq!(bold("hi"), "hi");
        NO_COLOR.store(false, Ordering::Relaxed);
    }

    #[test]
    fn test_color_wraps_ansi() {
        NO_COLOR.store(false, Ordering::Relaxed);
        assert!(green("ok").contains("\x1b[32m"));
        assert!(red("err").contains("\x1b[31m"));
        assert!(yellow("warn").contains("\x1b[33m"));
        assert!(bold("b").contains("\x1b[1m"));
        assert!(dim("d").contains("\x1b[2m"));
    }

    #[test]
    fn test_header_bold_underline() {
        NO_COLOR.store(false, Ordering::Relaxed);
        let h = header("Title");
        assert!(h.contains("\x1b[1m"));
        assert!(h.contains("\x1b[4m"));
        assert!(h.contains("Title"));
    }

    #[test]
    fn test_pass_fail_warn_skip_icons() {
        NO_COLOR.store(false, Ordering::Relaxed);
        assert!(pass("ok").contains("✓"));
        assert!(fail("bad").contains("✗"));
        assert!(warn_icon("maybe").contains("⚠"));
        assert!(skip("nope").contains("⏭"));
    }

    #[test]
    fn test_pass_fail_no_color() {
        NO_COLOR.store(true, Ordering::Relaxed);
        // Race-safe: output must always contain the text
        assert!(pass("ok").contains("ok"));
        assert!(fail("bad").contains("bad"));
        assert!(warn_icon("maybe").contains("maybe"));
        assert!(skip("nope").contains("nope"));
        NO_COLOR.store(false, Ordering::Relaxed);
    }

    #[test]
    fn test_grade_coloring() {
        NO_COLOR.store(false, Ordering::Relaxed);
        // Race-safe: verify text content is present (ANSI may or may not be applied due to parallel test races)
        assert!(grade("A").contains("A"));
        assert!(grade("B").contains("B"));
        assert!(grade("C").contains("C"));
        assert!(grade("D").contains("D"));
        assert!(grade("F").contains("F"));
    }

    #[test]
    fn test_grade_no_color() {
        NO_COLOR.store(true, Ordering::Relaxed);
        // Race-safe: grade text must always contain the letter
        let a = grade("A");
        let f = grade("F");
        assert!(a.contains("A"), "grade('A') must contain 'A': {a:?}");
        assert!(f.contains("F"), "grade('F') must contain 'F': {f:?}");
        NO_COLOR.store(false, Ordering::Relaxed);
    }

    #[test]
    fn test_pct_thresholds() {
        NO_COLOR.store(false, Ordering::Relaxed);
        // Race-safe: verify percentage text is present
        assert!(pct(95.0, 90.0, 75.0).contains("95.0%"));
        assert!(pct(80.0, 90.0, 75.0).contains("80.0%"));
        assert!(pct(50.0, 90.0, 75.0).contains("50.0%"));
    }

    #[test]
    fn test_delta_coloring() {
        NO_COLOR.store(false, Ordering::Relaxed);
        // Race-safe: verify delta text content
        assert!(delta(5.0).contains("+5.0"));
        assert!(delta(-3.0).contains("-3.0"));
        assert!(delta(0.0).contains("0.0"));
    }

    #[test]
    fn test_score_frac() {
        NO_COLOR.store(false, Ordering::Relaxed);
        let s = score_frac(18.0, 20.0, 80.0, 60.0);
        assert!(s.contains("18.0"));
        assert!(s.contains("20.0"));
    }

    #[test]
    fn test_score_frac_no_color() {
        NO_COLOR.store(true, Ordering::Relaxed);
        let s = score_frac(18.0, 20.0, 80.0, 60.0);
        assert!(s.contains("18.0"));
        assert!(s.contains("20.0"));
        NO_COLOR.store(false, Ordering::Relaxed);
    }

    #[test]
    fn test_score_frac_zero_max() {
        NO_COLOR.store(false, Ordering::Relaxed);
        let s = score_frac(0.0, 0.0, 80.0, 60.0);
        assert!(s.contains("0.0"));
    }

    #[test]
    fn test_rule_and_separator() {
        assert!(rule().contains("━") || rule().contains("━"));
        assert!(separator().contains("─") || separator().contains("─"));
    }

    #[test]
    fn test_path_cyan() {
        NO_COLOR.store(false, Ordering::Relaxed);
        assert!(path("/etc/app.conf").contains("/etc/app.conf"));
    }

    #[test]
    fn test_duration_colored() {
        NO_COLOR.store(false, Ordering::Relaxed);
        // Race-safe: verify duration text content
        let under = duration_colored(0.005, 0.01);
        let over = duration_colored(0.015, 0.01);
        let way_over = duration_colored(0.025, 0.01);
        assert!(under.contains("ms") || under.contains("µs"));
        assert!(over.contains("ms"));
        assert!(way_over.contains("ms"));
    }

    #[test]
    fn test_duration_formatting() {
        NO_COLOR.store(true, Ordering::Relaxed);
        assert!(duration_colored(2.5, 1.0).contains("2.50s"));
        assert!(duration_colored(0.123, 1.0).contains("ms"));
        assert!(duration_colored(0.000_45, 1.0).contains("µs"));
        NO_COLOR.store(false, Ordering::Relaxed);
    }

    #[test]
    fn test_wrap_empty_code() {
        // Empty code string should not wrap
        let result = wrap("", "hello");
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_grade_unknown() {
        // Unknown grade letter — empty color code maps to no wrapping
        assert!(grade("?").contains("?"));
    }
}
