//! The four checks. Each iterates and pushes; none has an error path.
//!
//! Three of them wire up machinery that already shipped — `core::purifier`
//! (bashrs), `core::script_secret_lint`, `core::compliance_gate` — rather than
//! restating it. The genuinely new one is the cyclomatic ceiling, which comes
//! free from bashrs 6.68's CFG builder, already in the dependency graph.

use super::{GateFinding, GateLevel, GateThresholds};
use crate::core::script_secret_lint::ScriptLeakFinding;
use crate::core::types::{ForjarConfig, PolicyViolation, Resource};
use crate::core::{codegen, compliance_gate, script_secret_lint, secrets};
use std::collections::HashSet;

/// One generated script, with the resource and phase it came from.
pub struct GeneratedScript {
    /// Resource id the script was emitted for.
    pub resource_id: String,
    /// `"check"` | `"apply"` | `"state_query"`.
    pub kind: &'static str,
    /// The emitted shell.
    pub text: String,
}

/// Emit every generated script ONCE, for all three script-shaped checks.
///
/// Codegen is not free, and the checks that follow used to each re-emit the
/// same three scripts per resource.
pub fn generate_scripts(config: &ForjarConfig) -> Vec<GeneratedScript> {
    let mut out = Vec::new();
    for (id, resource) in &config.resources {
        for (kind, result) in [
            ("check", codegen::check_script(resource)),
            ("apply", codegen::apply_script(resource)),
            ("state_query", codegen::state_query_script(resource)),
        ] {
            if let Ok(text) = result {
                out.push(GeneratedScript {
                    resource_id: id.clone(),
                    kind,
                    text,
                });
            }
        }
    }
    out
}

// ── CHECK 1: cyclomatic ceiling over generated shell ────────────────

/// McCabe complexity of a generated script, or `None` if it does not parse.
///
/// A parse failure is deliberately NOT a complexity finding: that is check 3's
/// job, and reporting it twice under two rule ids would double-count one defect.
fn cyclomatic(script: &str) -> Option<usize> {
    let mut parser = bashrs::bash_parser::BashParser::new(script).ok()?;
    let ast = parser.parse().ok()?;
    let cfg = bashrs::quality::build_cfg_from_ast(&ast.statements);
    Some(bashrs::quality::ComplexityMetrics::from_cfg(&cfg).cyclomatic)
}

/// Flag generated scripts whose control flow exceeds the configured ceiling.
pub fn check_shell_complexity(
    scripts: &[GeneratedScript],
    thresholds: &GateThresholds,
    out: &mut Vec<GateFinding>,
) {
    let Some(max) = thresholds.max_cyclomatic else {
        return;
    };
    let level = if thresholds.complexity_is_error {
        GateLevel::Error
    } else {
        GateLevel::Warning
    };
    for script in scripts {
        let Some(value) = cyclomatic(&script.text) else {
            continue;
        };
        if value > max {
            out.push(
                GateFinding::new(
                    "FJQ-CPX-001",
                    level,
                    &script.resource_id,
                    // "approximately": bashrs computes E - N + 2P with P
                    // hardcoded to 1 ("single connected component for now"),
                    // so the number is a close bound, not an exact McCabe.
                    format!(
                        "generated {} script has cyclomatic complexity ~{value}, over the ceiling of {max}",
                        script.kind
                    ),
                )
                .in_script(script.kind, 1)
                .with_remediation(Some(
                    "the shell is emitted by forjar's codegen — split the resource, \
                     or raise the ceiling if the emitter is doing the right thing"
                        .to_string(),
                )),
            );
        }
    }
}

// ── CHECK 2: plaintext secrets ──────────────────────────────────────

/// Leak findings in `text`, minus any on a line that carries real ciphertext.
///
/// The suppression is what makes the verdict honest rather than a grep for the
/// word "password": `secrets::has_encrypted_markers` accepts only
/// `ENC[age,<>=20 chars of decodable base64>]`, so prose about ENC markers does
/// not suppress anything and a sealed value is not reported as plaintext.
fn unsealed_leaks(text: &str, skip_comments: bool) -> Vec<ScriptLeakFinding> {
    let lines: Vec<&str> = text.lines().collect();
    script_secret_lint::scan_text(text, skip_comments)
        .findings
        .into_iter()
        .filter(|f| {
            !lines
                .get(f.line.saturating_sub(1))
                .is_some_and(|l| secrets::has_encrypted_markers(l))
        })
        .collect()
}

/// FJQ-SEC-001: a secret that reaches a generated script.
fn scan_generated_scripts(scripts: &[GeneratedScript], out: &mut Vec<GateFinding>) {
    for script in scripts {
        for leak in unsealed_leaks(&script.text, true) {
            out.push(
                GateFinding::new(
                    "FJQ-SEC-001",
                    GateLevel::Error,
                    &script.resource_id,
                    format!(
                        "generated {} script leaks a secret ({}): {}",
                        script.kind, leak.pattern_name, leak.matched_text
                    ),
                )
                .in_script(script.kind, leak.line),
            );
        }
    }
}

/// The resource fields an operator writes a literal secret into.
fn secret_bearing_fields(resource: &Resource) -> Vec<(&'static str, &str)> {
    let mut fields = Vec::new();
    for (name, value) in [
        ("content", &resource.content),
        ("command", &resource.command),
        ("source", &resource.source),
    ] {
        if let Some(v) = value {
            fields.push((name, v.as_str()));
        }
    }
    for value in &resource.environment {
        fields.push(("environment", value.as_str()));
    }
    fields
}

/// FJQ-SEC-002: a secret sitting unencrypted in the config itself.
///
/// This is the half a script scan cannot see. `security_scanner::hardcoded_secrets`
/// looks at `content` alone and does not discriminate sealed values; this pass
/// covers `command`, `source` and `environment` too, and suppresses `ENC[age,…]`.
fn scan_config_fields(config: &ForjarConfig, out: &mut Vec<GateFinding>) {
    for (id, resource) in &config.resources {
        for (field, value) in secret_bearing_fields(resource) {
            for leak in unsealed_leaks(value, false) {
                out.push(GateFinding::new(
                    "FJQ-SEC-002",
                    GateLevel::Error,
                    id,
                    format!(
                        "unencrypted secret in `{field}` ({}): {} — seal it as ENC[age,…]",
                        leak.pattern_name, leak.matched_text
                    ),
                ));
            }
        }
    }
}

/// Both halves of the secret check.
pub fn check_plaintext_secrets(
    config: &ForjarConfig,
    scripts: &[GeneratedScript],
    out: &mut Vec<GateFinding>,
) {
    scan_generated_scripts(scripts, out);
    scan_config_fields(config, out);
}

// ── CHECK 3: shell injection / malformed shell (bashrs AST) ─────────

/// Line numbers that fall inside a heredoc body — file DATA, not shell.
///
/// Moved here verbatim from `cli/lint.rs`, which was the only surface applying
/// it. `mcp/handlers.rs` applied no such filter, so the same verb reported
/// heredoc content as shell defects over MCP and did not over the CLI.
pub fn heredoc_line_set(script: &str) -> HashSet<usize> {
    let mut inside = HashSet::new();
    let mut in_heredoc = false;
    for (i, line) in script.lines().enumerate() {
        let trimmed = line.trim();
        if in_heredoc {
            if trimmed == "FORJAR_EOF" || trimmed == "FORJAR_SUDO" {
                in_heredoc = false;
            } else {
                inside.insert(i + 1); // 1-based
            }
        } else if trimmed.contains("<<'FORJAR_EOF'") || trimmed.contains("<<'FORJAR_SUDO'") {
            in_heredoc = true;
        }
    }
    inside
}

/// Run bashrs over every generated script.
///
/// The `SC1*` family is NOT filtered. `cli/lint.rs` dropped it with a stale
/// note about false positives — the same exclusion `core::purifier` removed in
/// forjar#285 after linting 1,311 generated scripts and measuring ZERO SC1
/// hits. SC1 is the SYNTAX-error family, which is exactly what a gate over
/// shell forjar itself emits most wants to catch.
///
/// Diagnostics BELOW `Severity::Warning` (Info, Note, Perf, Risk) are dropped.
/// They are style advice about shell the OPERATOR did not write — a fleet of
/// 437 resources emits roughly four thousand "add a shebang" notes over its
/// own generated scripts — and an operator cannot act on any of them. Shipping
/// them to a SARIF consumer would bury the two findings that matter.
pub fn check_shell_injection(scripts: &[GeneratedScript], out: &mut Vec<GateFinding>) {
    use bashrs::linter::Severity;
    for script in scripts {
        // The SAME text `transport::validate_before_exec` judges. Linting the
        // raw script instead would have the gate refuse an apply over a base64
        // blob or a `content:` heredoc the executor accepts without comment.
        let sanitised = crate::transport::strip_data_payloads(&script.text);
        let heredoc = heredoc_line_set(&sanitised);
        for d in &crate::core::purifier::lint_script(&sanitised).diagnostics {
            if heredoc.contains(&d.span.start_line) || d.severity < Severity::Warning {
                continue;
            }
            let level = if d.severity == Severity::Error {
                GateLevel::Error
            } else {
                GateLevel::Warning
            };
            out.push(
                GateFinding::new(
                    format!("FJQ-SH-{}", d.code),
                    level,
                    &script.resource_id,
                    d.message.clone(),
                )
                .in_script(script.kind, d.span.start_line),
            );
        }
    }
}

// ── CHECK 4: compliance ─────────────────────────────────────────────

/// Project one in-config `policies:` violation onto a gate finding.
///
/// Public because `parser::policy::policy_check_to_sarif` routes through it,
/// so the repo has ONE SARIF emitter rather than two that can drift.
pub fn violation_to_finding(v: &PolicyViolation) -> GateFinding {
    use crate::core::types::PolicySeverity;
    let rule_id = v
        .policy_id
        .clone()
        .unwrap_or_else(|| format!("forjar/{:?}", v.rule_type).to_lowercase());
    let level = match v.severity {
        PolicySeverity::Error => GateLevel::Error,
        PolicySeverity::Warning => GateLevel::Warning,
        PolicySeverity::Info => GateLevel::Note,
    };
    GateFinding::new(rule_id, level, &v.resource_id, v.rule_message.clone())
        .with_remediation(v.remediation.clone())
}

/// Evaluate compliance packs from `policy_dir`, then in-config `policies:`.
pub fn check_compliance(
    config: &ForjarConfig,
    thresholds: &GateThresholds,
    out: &mut Vec<GateFinding>,
) {
    for v in &crate::core::parser::evaluate_policies_full(config).violations {
        out.push(violation_to_finding(v));
    }
    let Some(dir) = &thresholds.policy_dir else {
        return;
    };
    match compliance_gate::check_compliance_gate(dir, config, false) {
        Ok(result) => {
            for pack in &result.results {
                for rule in pack.results.iter().filter(|r| !r.passed) {
                    out.push(GateFinding::new(
                        rule.rule_id.clone(),
                        GateLevel::from_severity_str(&rule.severity),
                        "",
                        format!("{}: {}", pack.pack_name, rule.message),
                    ));
                }
            }
        }
        // A gate that cannot evaluate its packs must not silently pass: an
        // unreadable policy directory would otherwise read as compliant.
        //
        // This arm was UNREACHABLE until #356. `compliance_pack::list_packs`
        // answered `Vec::new()` for a directory it could not read, so
        // `check_compliance_gate` returned `Ok` with zero packs and the gate
        // passed — the comment above stated the intent while the code did the
        // opposite. Two things reach it now, and both are BLINDNESS rather than
        // a rule verdict: a directory that will not list, and a pack file that
        // is present and will not read. A file that reads and does not parse as
        // a pack is forjar's own `*.yaml` guess being wrong, and is skipped
        // rather than blocking — see `compliance_gate::check_compliance_gate`.
        Err(e) => out.push(
            GateFinding::new(
                "FJQ-CMP-000",
                GateLevel::Error,
                "",
                format!("compliance packs could not be evaluated: {e}"),
            )
            .with_remediation(Some(
                "make --policy-dir and every pack under it readable, or drop the \
                 flag — the gate cannot report compliant over packs it could not see"
                    .to_string(),
            )),
        ),
    }
}
