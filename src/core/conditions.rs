//! FJ-202: Conditional resource evaluation.
//!
//! Evaluates `when:` expressions on resources. Expressions use template
//! variables (`{{params.*}}`, `{{machine.*}}`) and comparison operators
//! (`==`, `!=`, `contains`). Evaluated per-machine at plan time — false
//! resources are excluded from the execution plan.

use super::types::*;
use std::collections::HashMap;

/// Resolve templates in a `when:` expression using the current machine context.
///
/// Unlike the global resolver, `{{machine.FIELD}}` here refers to the
/// *current* machine being evaluated (no machine name in the path).
fn resolve_when_template(
    template: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machine: &Machine,
) -> Result<String, String> {
    let mut result = template.to_string();
    let mut start = 0;

    while let Some(open) = result[start..].find("{{") {
        let open = start + open;
        let close = result[open..]
            .find("}}")
            .ok_or_else(|| format!("unclosed template in when expression at position {open}"))?;
        let close = open + close + 2;
        let key = result[open + 2..close - 2].trim();

        let value = if let Some(param_key) = key.strip_prefix("params.") {
            params
                .get(param_key)
                .map(yaml_value_to_string)
                .ok_or_else(|| format!("unknown param in when expression: {param_key}"))?
        } else if let Some(field) = key.strip_prefix("machine.") {
            match field {
                "arch" => machine.arch.clone(),
                "hostname" => machine.hostname.clone(),
                "addr" => machine.addr.clone(),
                "user" => machine.user.clone(),
                "roles" => format!("{:?}", machine.roles),
                _ => return Err(format!("unknown machine field in when expression: {field}")),
            }
        } else {
            return Err(format!(
                "unknown template variable in when expression: {key}"
            ));
        };

        result.replace_range(open..close, &value);
        start = open + value.len();
    }

    Ok(result)
}

/// Strip surrounding quotes from a string value.
fn unquote(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Evaluate a resolved `when:` expression to a boolean.
///
/// Supports:
/// - `true` / `false` literals
/// - `LEFT == RIGHT` (string equality)
/// - `LEFT != RIGHT` (string inequality)
/// - `LEFT contains RIGHT` (substring/list membership)
fn evaluate_expression(resolved: &str) -> Result<bool, String> {
    let trimmed = resolved.trim();

    // Literal booleans
    if trimmed.eq_ignore_ascii_case("true") {
        return Ok(true);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return Ok(false);
    }

    // Try operators in order of specificity
    if let Some(idx) = trimmed.find(" != ") {
        let left = unquote(&trimmed[..idx]);
        let right = unquote(&trimmed[idx + 4..]);
        return Ok(left != right);
    }

    if let Some(idx) = trimmed.find(" == ") {
        let left = unquote(&trimmed[..idx]);
        let right = unquote(&trimmed[idx + 4..]);
        return Ok(left == right);
    }

    if let Some(idx) = trimmed.find(" contains ") {
        let left = unquote(&trimmed[..idx]);
        let right = unquote(&trimmed[idx + 10..]);
        return Ok(left.contains(right));
    }

    Err(format!(
        "invalid when expression: '{trimmed}' (expected: EXPR == VALUE, EXPR != VALUE, EXPR contains VALUE, true, or false)"
    ))
}

/// Evaluate a `when:` condition for a resource on a specific machine.
///
/// Returns `true` if the resource should be applied (condition met or no condition).
/// Returns `false` if the condition evaluates to false (skip this resource).
pub fn evaluate_when(
    when_expr: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machine: &Machine,
) -> Result<bool, String> {
    let resolved = resolve_when_template(when_expr, params, machine)?;
    evaluate_expression(&resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_machine(arch: &str) -> Machine {
        Machine {
            hostname: "test-host".to_string(),
            addr: "10.0.0.1".to_string(),
            user: "root".to_string(),
            arch: arch.to_string(),
            ssh_key: None,
            roles: vec!["web".to_string(), "gpu".to_string()],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        }
    }

    fn make_params() -> HashMap<String, serde_yaml_ng::Value> {
        let mut params = HashMap::new();
        params.insert(
            "env".to_string(),
            serde_yaml_ng::Value::String("production".to_string()),
        );
        params.insert("feature_flag".to_string(), serde_yaml_ng::Value::Bool(true));
        params
    }

    // ── Literal tests ──────────────────────────────────────────────

    #[test]
    fn test_fj202_literal_true() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(evaluate_when("true", &p, &m).unwrap());
    }

    #[test]
    fn test_fj202_literal_false() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(!evaluate_when("false", &p, &m).unwrap());
    }

    #[test]
    fn test_fj202_literal_true_case_insensitive() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(evaluate_when("TRUE", &p, &m).unwrap());
        assert!(!evaluate_when("FALSE", &p, &m).unwrap());
    }

    // ── Equality tests ─────────────────────────────────────────────

    #[test]
    fn test_fj202_arch_equals() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(evaluate_when("{{machine.arch}} == \"x86_64\"", &p, &m).unwrap());
    }

    #[test]
    fn test_fj202_arch_not_equals() {
        let m = make_machine("aarch64");
        let p = HashMap::new();
        assert!(!evaluate_when("{{machine.arch}} == \"x86_64\"", &p, &m).unwrap());
    }

    #[test]
    fn test_fj202_param_equals() {
        let m = make_machine("x86_64");
        let p = make_params();
        assert!(evaluate_when("{{params.env}} == \"production\"", &p, &m).unwrap());
    }

    #[test]
    fn test_fj202_param_not_equals() {
        let m = make_machine("x86_64");
        let p = make_params();
        assert!(evaluate_when("{{params.env}} != \"staging\"", &p, &m).unwrap());
    }

    #[test]
    fn test_fj202_param_not_equals_false() {
        let m = make_machine("x86_64");
        let p = make_params();
        assert!(!evaluate_when("{{params.env}} != \"production\"", &p, &m).unwrap());
    }

    // ── Contains tests ─────────────────────────────────────────────

    #[test]
    fn test_fj202_roles_contains() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(evaluate_when("{{machine.roles}} contains \"gpu\"", &p, &m).unwrap());
    }

    #[test]
    fn test_fj202_roles_not_contains() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(!evaluate_when("{{machine.roles}} contains \"storage\"", &p, &m).unwrap());
    }

    // ── Error tests ────────────────────────────────────────────────

    #[test]
    fn test_fj202_unknown_param() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        let result = evaluate_when("{{params.missing}} == \"x\"", &p, &m);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown param"));
    }

    #[test]
    fn test_fj202_unknown_machine_field() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        let result = evaluate_when("{{machine.nonexistent}} == \"x\"", &p, &m);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown machine field"));
    }

    #[test]
    fn test_fj202_invalid_operator() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        let result = evaluate_when("something without operator", &p, &m);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid when expression"));
    }

    #[test]
    fn test_fj202_unclosed_template() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        let result = evaluate_when("{{machine.arch == \"x\"", &p, &m);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unclosed template"));
    }

    // ── Machine field tests ────────────────────────────────────────

    #[test]
    fn test_fj202_hostname_equals() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(evaluate_when("{{machine.hostname}} == \"test-host\"", &p, &m).unwrap());
    }

    #[test]
    fn test_fj202_addr_equals() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(evaluate_when("{{machine.addr}} == \"10.0.0.1\"", &p, &m).unwrap());
    }

    #[test]
    fn test_fj202_user_equals() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(evaluate_when("{{machine.user}} == \"root\"", &p, &m).unwrap());
    }

    // ── Boolean param test ─────────────────────────────────────────

    #[test]
    fn test_fj202_bool_param() {
        let m = make_machine("x86_64");
        let p = make_params();
        assert!(evaluate_when("{{params.feature_flag}} == \"true\"", &p, &m).unwrap());
    }

    // ── Whitespace handling ────────────────────────────────────────

    #[test]
    fn test_fj202_whitespace_trim() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(evaluate_when("  true  ", &p, &m).unwrap());
    }

    #[test]
    fn test_fj202_single_quoted_values() {
        let m = make_machine("x86_64");
        let p = HashMap::new();
        assert!(evaluate_when("{{machine.arch}} == 'x86_64'", &p, &m).unwrap());
    }
}
