//! FJ-3207: SARIF output tests.

use super::*;

#[test]
fn sarif_empty_violations() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packages: [curl]
"#;
    let config = parse_config(yaml).unwrap();
    let result = evaluate_policies_full(&config);
    let sarif = policy_check_to_sarif(&result);

    let parsed: serde_json::Value = serde_json::from_str(&sarif).unwrap();
    assert_eq!(parsed["version"], "2.1.0");
    assert_eq!(parsed["runs"][0]["results"].as_array().unwrap().len(), 0);
}

#[test]
fn sarif_with_violations() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
policies:
  - type: require
    message: "files must have owner"
    id: "SEC-001"
    field: owner
    remediation: "Add owner field"
"#;
    let config = parse_config(yaml).unwrap();
    let result = evaluate_policies_full(&config);
    let sarif = policy_check_to_sarif(&result);

    let parsed: serde_json::Value = serde_json::from_str(&sarif).unwrap();
    assert_eq!(parsed["version"], "2.1.0");

    let run = &parsed["runs"][0];
    assert_eq!(run["tool"]["driver"]["name"], "forjar");

    let rules = run["tool"]["driver"]["rules"].as_array().unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["id"], "SEC-001");

    let results = run["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["ruleId"], "SEC-001");
    assert_eq!(results[0]["level"], "error");
    assert!(results[0]["message"]["text"]
        .as_str()
        .unwrap()
        .contains("files must have owner"));
}

#[test]
fn sarif_severity_mapping() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    owner: root
policies:
  - type: warn
    message: "avoid root owner"
    id: "WARN-001"
    condition_field: owner
    condition_value: root
  - type: deny
    message: "root owner denied"
    id: "ERR-001"
    condition_field: owner
    condition_value: root
    severity: info
"#;
    let config = parse_config(yaml).unwrap();
    let result = evaluate_policies_full(&config);
    let sarif = policy_check_to_sarif(&result);

    let parsed: serde_json::Value = serde_json::from_str(&sarif).unwrap();
    let results = parsed["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);

    // Warn type → warning severity → SARIF "warning"
    let warn_result = results.iter().find(|r| r["ruleId"] == "WARN-001").unwrap();
    assert_eq!(warn_result["level"], "warning");

    // Deny with severity override to info → SARIF "note"
    let info_result = results.iter().find(|r| r["ruleId"] == "ERR-001").unwrap();
    assert_eq!(info_result["level"], "note");
}

#[test]
fn sarif_schema_url() {
    let yaml = r#"
version: "1.0"
name: test
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let result = evaluate_policies_full(&config);
    let sarif = policy_check_to_sarif(&result);

    let parsed: serde_json::Value = serde_json::from_str(&sarif).unwrap();
    assert!(parsed["$schema"]
        .as_str()
        .unwrap()
        .contains("sarif-schema-2.1.0"));
}
