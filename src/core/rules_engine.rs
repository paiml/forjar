//! FJ-3108: Rulebook validation engine.
//!
//! Validates rulebook YAML: event types, action completeness,
//! cooldown bounds, and cross-references.

use crate::core::types::{EventType, Rulebook, RulebookAction, RulebookConfig};
use std::collections::HashSet;
use std::path::Path;

/// A validation issue found in a rulebook.
#[derive(Debug, Clone)]
pub struct RuleIssue {
    /// Rulebook name where the issue was found.
    pub rulebook: String,
    /// Severity level.
    pub severity: IssueSeverity,
    /// Human-readable description.
    pub message: String,
}

/// Severity of a rulebook validation issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Blocks promotion.
    Error,
    /// Advisory — does not block.
    Warning,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
        }
    }
}

/// Validate a rulebook config file (YAML parse + semantic checks).
pub fn validate_rulebook_file(path: &Path) -> Result<Vec<RuleIssue>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    validate_rulebook_yaml(&content)
}

/// Validate rulebook YAML content.
pub fn validate_rulebook_yaml(content: &str) -> Result<Vec<RuleIssue>, String> {
    let config: RulebookConfig =
        serde_yaml_ng::from_str(content).map_err(|e| format!("YAML parse error: {e}"))?;
    Ok(validate_rulebook_config(&config))
}

/// Validate a parsed rulebook config.
pub fn validate_rulebook_config(config: &RulebookConfig) -> Vec<RuleIssue> {
    let mut issues = Vec::new();
    let mut seen_names = HashSet::new();

    for rb in &config.rulebooks {
        // Duplicate name check
        if !seen_names.insert(&rb.name) {
            issues.push(RuleIssue {
                rulebook: rb.name.clone(),
                severity: IssueSeverity::Error,
                message: format!("duplicate rulebook name: {}", rb.name),
            });
        }

        validate_single_rulebook(rb, &mut issues);
    }

    issues
}

/// Validate a single rulebook entry.
fn validate_single_rulebook(rb: &Rulebook, issues: &mut Vec<RuleIssue>) {
    // Must have at least one event pattern
    if rb.events.is_empty() {
        issues.push(RuleIssue {
            rulebook: rb.name.clone(),
            severity: IssueSeverity::Error,
            message: "no event patterns defined".into(),
        });
    }

    // Must have at least one action
    if rb.actions.is_empty() {
        issues.push(RuleIssue {
            rulebook: rb.name.clone(),
            severity: IssueSeverity::Error,
            message: "no actions defined".into(),
        });
    }

    // Validate each action
    for (i, action) in rb.actions.iter().enumerate() {
        validate_action(rb, action, i, issues);
    }

    // Cooldown warnings
    if rb.cooldown_secs == 0 {
        issues.push(RuleIssue {
            rulebook: rb.name.clone(),
            severity: IssueSeverity::Warning,
            message: "cooldown_secs=0 may cause rapid-fire triggering".into(),
        });
    }

    // Max retries sanity
    if rb.max_retries > 10 {
        issues.push(RuleIssue {
            rulebook: rb.name.clone(),
            severity: IssueSeverity::Warning,
            message: format!("max_retries={} is unusually high", rb.max_retries),
        });
    }
}

/// Validate a single rulebook action.
fn validate_action(
    rb: &Rulebook,
    action: &RulebookAction,
    idx: usize,
    issues: &mut Vec<RuleIssue>,
) {
    let action_count = [
        action.apply.is_some(),
        action.destroy.is_some(),
        action.script.is_some(),
        action.notify.is_some(),
    ]
    .iter()
    .filter(|&&b| b)
    .count();

    if action_count == 0 {
        issues.push(RuleIssue {
            rulebook: rb.name.clone(),
            severity: IssueSeverity::Error,
            message: format!("action[{idx}] has no action type configured"),
        });
    }

    if action_count > 1 {
        issues.push(RuleIssue {
            rulebook: rb.name.clone(),
            severity: IssueSeverity::Warning,
            message: format!(
                "action[{idx}] has multiple action types; only the first will execute"
            ),
        });
    }

    // Validate apply action has a file
    if let Some(ref apply) = action.apply {
        if apply.file.is_empty() {
            issues.push(RuleIssue {
                rulebook: rb.name.clone(),
                severity: IssueSeverity::Error,
                message: format!("action[{idx}] apply.file is empty"),
            });
        }
    }

    // Validate script is non-empty and passes bashrs + secret lint
    if let Some(ref script) = action.script {
        if script.trim().is_empty() {
            issues.push(RuleIssue {
                rulebook: rb.name.clone(),
                severity: IssueSeverity::Warning,
                message: format!("action[{idx}] script is empty"),
            });
        } else {
            // FJ-3108/3204: bashrs purification check
            if let Err(e) = crate::core::purifier::validate_script(script) {
                issues.push(RuleIssue {
                    rulebook: rb.name.clone(),
                    severity: IssueSeverity::Error,
                    message: format!("action[{idx}] bashrs lint failed: {e}"),
                });
            }
            // FJ-3307: secret leak detection
            if let Err(e) = crate::core::script_secret_lint::validate_no_leaks(script) {
                issues.push(RuleIssue {
                    rulebook: rb.name.clone(),
                    severity: IssueSeverity::Error,
                    message: format!("action[{idx}] secret leak: {e}"),
                });
            }
        }
    }

    // Validate notify has non-empty channel
    if let Some(ref notify) = action.notify {
        if notify.channel.is_empty() {
            issues.push(RuleIssue {
                rulebook: rb.name.clone(),
                severity: IssueSeverity::Error,
                message: format!("action[{idx}] notify.channel is empty"),
            });
        }
    }
}

/// Summary of rulebook validation.
#[derive(Debug, Clone)]
pub struct ValidationSummary {
    /// Total rulebooks validated.
    pub rulebook_count: usize,
    /// Total issues found.
    pub issues: Vec<RuleIssue>,
}

impl ValidationSummary {
    /// Create from issues and rulebook count.
    pub fn new(rulebook_count: usize, issues: Vec<RuleIssue>) -> Self {
        Self {
            rulebook_count,
            issues,
        }
    }

    /// Number of errors.
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .count()
    }

    /// Number of warnings.
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Warning)
            .count()
    }

    /// Whether validation passed (no errors).
    pub fn passed(&self) -> bool {
        self.error_count() == 0
    }
}

/// Count event types used across all rulebooks.
pub fn event_type_coverage(config: &RulebookConfig) -> Vec<(EventType, usize)> {
    let all_types = [
        EventType::FileChanged,
        EventType::ProcessExit,
        EventType::CronFired,
        EventType::WebhookReceived,
        EventType::MetricThreshold,
        EventType::Manual,
    ];

    all_types
        .iter()
        .map(|et| {
            let count = config
                .rulebooks
                .iter()
                .flat_map(|rb| &rb.events)
                .filter(|ep| ep.event_type == *et)
                .count();
            (et.clone(), count)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_yaml() -> &'static str {
        r#"
rulebooks:
  - name: config-repair
    events:
      - type: file_changed
        match:
          path: /etc/nginx/nginx.conf
    actions:
      - apply:
          file: forjar.yaml
          tags: [config]
    cooldown_secs: 60
"#
    }

    #[test]
    fn validate_valid_rulebook() {
        let issues = validate_rulebook_yaml(valid_yaml()).unwrap();
        assert!(issues.is_empty(), "unexpected issues: {issues:?}");
    }

    #[test]
    fn validate_parse_error() {
        let result = validate_rulebook_yaml("not: valid: [yaml");
        assert!(result.is_err());
    }

    #[test]
    fn validate_no_events() {
        let yaml = r#"
rulebooks:
  - name: bad
    events: []
    actions:
      - script: "echo ok"
"#;
        let issues = validate_rulebook_yaml(yaml).unwrap();
        assert!(issues.iter().any(|i| i.message.contains("no event")));
    }

    #[test]
    fn validate_no_actions() {
        let yaml = r#"
rulebooks:
  - name: bad
    events:
      - type: manual
    actions: []
"#;
        let issues = validate_rulebook_yaml(yaml).unwrap();
        assert!(issues.iter().any(|i| i.message.contains("no actions")));
    }

    #[test]
    fn validate_duplicate_names() {
        let yaml = r#"
rulebooks:
  - name: dupe
    events: [{type: manual}]
    actions: [{script: "echo 1"}]
  - name: dupe
    events: [{type: manual}]
    actions: [{script: "echo 2"}]
"#;
        let issues = validate_rulebook_yaml(yaml).unwrap();
        assert!(issues.iter().any(|i| i.message.contains("duplicate")));
    }

    #[test]
    fn validate_empty_apply_file() {
        let yaml = r#"
rulebooks:
  - name: bad-apply
    events: [{type: manual}]
    actions:
      - apply:
          file: ""
"#;
        let issues = validate_rulebook_yaml(yaml).unwrap();
        assert!(issues
            .iter()
            .any(|i| i.message.contains("apply.file is empty")));
    }

    #[test]
    fn validate_zero_cooldown_warning() {
        let yaml = r#"
rulebooks:
  - name: rapid
    events: [{type: manual}]
    actions: [{script: "echo ok"}]
    cooldown_secs: 0
"#;
        let issues = validate_rulebook_yaml(yaml).unwrap();
        assert!(issues.iter().any(|i| {
            i.severity == IssueSeverity::Warning && i.message.contains("cooldown_secs=0")
        }));
    }

    #[test]
    fn validate_high_retries_warning() {
        let yaml = r#"
rulebooks:
  - name: retry
    events: [{type: manual}]
    actions: [{script: "echo ok"}]
    max_retries: 50
"#;
        let issues = validate_rulebook_yaml(yaml).unwrap();
        assert!(issues.iter().any(|i| i.message.contains("unusually high")));
    }

    #[test]
    fn validation_summary() {
        let issues = vec![
            RuleIssue {
                rulebook: "a".into(),
                severity: IssueSeverity::Error,
                message: "err".into(),
            },
            RuleIssue {
                rulebook: "b".into(),
                severity: IssueSeverity::Warning,
                message: "warn".into(),
            },
        ];
        let summary = ValidationSummary::new(2, issues);
        assert_eq!(summary.error_count(), 1);
        assert_eq!(summary.warning_count(), 1);
        assert!(!summary.passed());
    }

    #[test]
    fn validation_summary_passed() {
        let summary = ValidationSummary::new(1, vec![]);
        assert!(summary.passed());
        assert_eq!(summary.error_count(), 0);
    }

    #[test]
    fn event_type_coverage_counts() {
        let yaml = r#"
rulebooks:
  - name: r1
    events:
      - {type: file_changed}
      - {type: manual}
    actions: [{script: "echo 1"}]
  - name: r2
    events:
      - {type: file_changed}
    actions: [{script: "echo 2"}]
"#;
        let config: RulebookConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let coverage = event_type_coverage(&config);
        let fc = coverage
            .iter()
            .find(|(et, _)| *et == EventType::FileChanged);
        assert_eq!(fc.unwrap().1, 2);
        let m = coverage.iter().find(|(et, _)| *et == EventType::Manual);
        assert_eq!(m.unwrap().1, 1);
        let cr = coverage.iter().find(|(et, _)| *et == EventType::CronFired);
        assert_eq!(cr.unwrap().1, 0);
    }

    #[test]
    fn validate_file_not_found() {
        let result = validate_rulebook_file(Path::new("/nonexistent/file.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn validate_file_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.yaml");
        std::fs::write(&path, valid_yaml()).unwrap();
        let issues = validate_rulebook_file(&path).unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn empty_notify_channel() {
        let yaml = r#"
rulebooks:
  - name: bad-notify
    events: [{type: manual}]
    actions:
      - notify:
          channel: ""
          message: "test"
"#;
        let issues = validate_rulebook_yaml(yaml).unwrap();
        assert!(issues
            .iter()
            .any(|i| i.message.contains("notify.channel is empty")));
    }

    #[test]
    fn issue_severity_display() {
        assert_eq!(IssueSeverity::Error.to_string(), "error");
        assert_eq!(IssueSeverity::Warning.to_string(), "warning");
    }
}
