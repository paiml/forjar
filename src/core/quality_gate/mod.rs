//! The forjar quality gate: one aggregator, four checks, one SARIF projection.
//!
//! Three of the four checks already shipped, each reachable from exactly one
//! surface: bashrs shell linting from `forjar lint` AND `forjar_lint` (with
//! two DIFFERENT filters, so the same verb gave two answers), plaintext-secret
//! scanning from the shell provider only, compliance packs from `forjar apply
//! --policy-check` only. This module is where they meet, so that a verdict is
//! computed once and rendered by whichever transport asked for it.
//!
//! Where the gate lives is the load-bearing decision. It is NOT on the MCP
//! boundary: a gate that only the MCP handler runs makes `forjar lint` and
//! `forjar_lint` answer differently on identical input, which is the exact
//! divergence `src/verb/` exists to prevent. It sits in core, and both the CLI
//! leaf and the verb handler call it.
//!
//! It is also NOT in front of `validate` or `plan`. `validate` exists to answer
//! "is this config valid, and if not, why"; refusing to answer for a config
//! that fails a gate removes the only diagnostic an operator has exactly when
//! they need it. `plan` is `Effects::ReadOnly` and changes nothing, so a gate
//! there buys no safety while blocking the route to a fix. Enforcement belongs
//! in front of mutation, and that is `apply`.

use crate::core::types::ForjarConfig;
use serde::Serialize;
use std::path::PathBuf;

pub mod checks;
pub mod locate;
pub mod sarif;

#[cfg(test)]
#[path = "tests_gate.rs"]
mod tests_gate;

/// The error code an enforcement point reports when the gate blocks.
///
/// Named FORJAR, not PMAT: the epic that asked for this feature specified
/// `PMAT_QUALITY_GATE_VIOLATION`, but `pmat` appears in neither `Cargo.toml`
/// nor `Cargo.lock`. A constant that implies a dependency which does not exist
/// is a lie told to whoever greps for it next.
pub const QUALITY_GATE_ERROR_CODE: &str = "FORJAR_QUALITY_GATE_VIOLATION";

/// How much a finding matters. Only [`GateLevel::Error`] blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GateLevel {
    /// Blocks an enforcement point.
    Error,
    /// Reported, never blocking.
    Warning,
    /// Informational.
    Note,
}

impl GateLevel {
    /// The SARIF 2.1.0 `level` string. Derived, never restated at the emitter.
    pub const fn sarif_level(self) -> &'static str {
        match self {
            GateLevel::Error => "error",
            GateLevel::Warning => "warning",
            GateLevel::Note => "note",
        }
    }

    /// Parse a compliance pack's `severity:` string. Unknown maps to Warning,
    /// which is what `default_severity()` already gives an unlabelled rule.
    pub fn from_severity_str(s: &str) -> Self {
        match s {
            "error" => GateLevel::Error,
            "info" => GateLevel::Note,
            _ => GateLevel::Warning,
        }
    }
}

/// One thing the gate found.
#[derive(Debug, Clone, PartialEq)]
pub struct GateFinding {
    /// `FJQ-CPX-001` | `FJQ-SEC-001` | `FJQ-SEC-002` | `FJQ-SH-<code>` | a policy id.
    pub rule_id: String,
    /// Blocking or not.
    pub level: GateLevel,
    /// Resource this is about. Empty for a config-wide compliance rule.
    pub resource_id: String,
    /// Human-readable, WITHOUT the resource prefix — the renderer adds that.
    pub message: String,
    /// Fix advice, when the rule carries one. Becomes SARIF `help.text`.
    pub remediation: Option<String>,
    /// 1-based line of the resource key in the addressed YAML file.
    ///
    /// `None` is honest and common: a resource that arrived through
    /// `includes:` is not in the file the finding is addressed to, and the
    /// parsed model carries no spans to recover one from. A SARIF result with
    /// no line is correct; an invented line is not.
    pub yaml_line: Option<usize>,
    /// `"check"` | `"apply"` | `"state_query"` when the finding is about a
    /// generated script rather than the config text.
    pub script_kind: Option<&'static str>,
    /// 1-based line within that generated script.
    pub script_line: Option<usize>,
}

impl GateFinding {
    /// A finding about a resource's config text.
    pub fn new(
        rule_id: impl Into<String>,
        level: GateLevel,
        resource_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            level,
            resource_id: resource_id.into(),
            message: message.into(),
            remediation: None,
            yaml_line: None,
            script_kind: None,
            script_line: None,
        }
    }

    /// Attach the generated-script coordinates this finding came from.
    pub fn in_script(mut self, kind: &'static str, line: usize) -> Self {
        self.script_kind = Some(kind);
        self.script_line = Some(line);
        self
    }

    /// Attach fix advice.
    pub fn with_remediation(mut self, r: Option<String>) -> Self {
        self.remediation = r;
        self
    }

    /// The one rendering both the CLI leaf and the MCP verb print, so the two
    /// surfaces cannot describe the same finding with different words.
    pub fn render(&self) -> String {
        match (self.resource_id.as_str(), self.script_kind) {
            ("", _) => format!("{}: {}", self.rule_id, self.message),
            (id, Some(kind)) => format!("{} {id}/{kind}: {}", self.rule_id, self.message),
            (id, None) => format!("{} {id}: {}", self.rule_id, self.message),
        }
    }
}

/// What the gate is configured to enforce.
///
/// The default is every check that can BLOCK, and nothing that cannot: shell
/// safety, plaintext secrets and in-config policies always run; the complexity
/// ceiling and the compliance-pack directory are opt-in.
#[derive(Debug, Clone, Default)]
pub struct GateThresholds {
    /// Cyclomatic ceiling for a generated script. `None` genuinely SKIPS the
    /// check — no parse, no CFG — because the parse is the expensive part and
    /// a 437-resource fleet is 1,311 parses per invocation.
    ///
    /// Off by default, and that is a measurement rather than timidity: the
    /// `state_query` script forjar emits for a bare one-line `file` resource
    /// already scores 13, so any ceiling low enough to catch a real outlier
    /// would fire on ordinary configs. Both surfaces expose it as an opt-in
    /// (`--max-cyclomatic N`, `max_cyclomatic`).
    pub max_cyclomatic: Option<usize>,
    /// Directory of compliance packs. `None` skips pack evaluation; in-config
    /// `policies:` are always evaluated.
    ///
    /// SETTING THIS RUNS SHELL. `compliance_pack` evaluates a rule of
    /// `type: script` by handing it to `sh -c`, so whoever sets this executes
    /// whatever the pack author wrote. Measured, not inferred: a pack rule
    /// whose script is `touch <path>` creates that file.
    ///
    /// # Who may set it
    ///
    /// Operator-facing leaves only — `forjar lint --policy-dir`, `forjar apply
    /// --policy-check --policy-dir`. It is NOT reachable from the unified verb
    /// surface, so it is not a field of `mcp::types::LintInput` and cannot be
    /// passed over MCP, `forjar verb call` or HTTP.
    ///
    /// That is the whole of the boundary, and it is the SAME statement made in
    /// `verb::spec::Effects` and `verb::registry`: an `Effects::ReadOnly` verb
    /// writes nothing anywhere — not to a fleet machine, and not to the machine
    /// running the verb — because `readOnlyHint: true` is derived from it and
    /// an agent decides from that hint alone whether to call unattended.
    ///
    /// A previous revision of this doc claimed instead that "ReadOnly is with
    /// respect to the FLEET, not the machine running the verb", citing
    /// `ambient_inputs` (forjar#244) as precedent. That claim is withdrawn on
    /// two counts. A contract published in `verb/spec.rs` and `verb/registry.rs`
    /// cannot be amended in a third module that neither of them reads. And the
    /// precedent was a DEFECT, not a licence: forjar#372 measured `forjar_plan`
    /// over stdio executing the config's `ambient_inputs`, `sops`/`op` and
    /// `output_equivalence` commands, and closed it — `core::unattended::
    /// sanitize_config` strips all three before an unattended plan. That is the
    /// same boundary this field draws, arrived at from the CONFIG's side rather
    /// than the caller's, and the two now say one thing.
    ///
    /// The #356 fixture saw no ambient command fire, which is why the withdrawal
    /// was originally argued the other way round; with no lock to compare
    /// against, `plan` never reached the staleness probe. That is a property of
    /// that fixture, not of the surface — `tests/
    /// falsification_readonly_surface_executes_nothing.rs` uses one that does
    /// reach it. `policy_dir` remains the only thing a CALLER could aim.
    ///
    /// The gate itself is still ONE evaluator: no check is split by transport,
    /// and both surfaces render an identical verdict for identical thresholds.
    /// What differs is which surface may hand it a `policy_dir` at all.
    pub policy_dir: Option<PathBuf>,
    /// Whether a complexity finding blocks. Default FALSE, deliberately: the
    /// shell it scores is EMITTED by `core::codegen`, not written by the
    /// operator, so a cyclomatic 21 is a defect in forjar's emitter. Blocking
    /// an operator's apply for it punishes the wrong party.
    pub complexity_is_error: bool,
}

/// Everything the gate found, plus what it looked at.
#[derive(Debug, Clone)]
pub struct GateReport {
    /// Findings, in check order.
    pub findings: Vec<GateFinding>,
    /// Generated scripts examined.
    pub scripts_analysed: usize,
    /// Resources examined.
    pub resources_checked: usize,
}

impl GateReport {
    /// The predicate an enforcement point blocks on.
    pub fn passed(&self) -> bool {
        self.error_count() == 0
    }

    /// Blocking findings.
    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.level == GateLevel::Error)
            .count()
    }

    /// Non-blocking findings.
    pub fn advisory_count(&self) -> usize {
        self.findings.len() - self.error_count()
    }

    /// Human-readable lines: every blocking finding, then a tally.
    ///
    /// Advisory findings are counted rather than listed — a real fleet config
    /// produces hundreds of bashrs warnings over generated shell, and burying
    /// the errors under them is how an operator learns to ignore the output.
    /// They are all still in `findings` and in the SARIF.
    pub fn render(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .findings
            .iter()
            .filter(|f| f.level == GateLevel::Error)
            .map(GateFinding::render)
            .collect();
        if !self.findings.is_empty() {
            lines.push(format!(
                "quality gate: {} error(s), {} advisory across {} resource(s), {} generated script(s)",
                self.error_count(),
                self.advisory_count(),
                self.resources_checked,
                self.scripts_analysed
            ));
        }
        lines
    }

    /// SARIF 2.1.0 projection of these findings.
    pub fn to_sarif(&self, artifact_uri: &str) -> serde_json::Value {
        sarif::findings_to_sarif(&self.findings, artifact_uri)
    }
}

/// Run every check and collect the verdict.
///
/// `yaml_text` is the raw bytes of the file the findings are addressed to. It
/// is optional because the gate is also called on configs assembled in memory;
/// when it is absent, findings simply carry no line number.
pub fn evaluate(
    config: &ForjarConfig,
    yaml_text: Option<&str>,
    thresholds: &GateThresholds,
) -> GateReport {
    let scripts = checks::generate_scripts(config);
    let mut findings = Vec::new();
    checks::check_shell_complexity(&scripts, thresholds, &mut findings);
    checks::check_plaintext_secrets(config, &scripts, &mut findings);
    checks::check_shell_injection(&scripts, &mut findings);
    checks::check_compliance(config, thresholds, &mut findings);
    if let Some(text) = yaml_text {
        locate::annotate(text, &mut findings);
    }
    GateReport {
        findings,
        scripts_analysed: scripts.len(),
        resources_checked: config.resources.len(),
    }
}
