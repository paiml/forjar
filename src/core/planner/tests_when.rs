use super::tests_helpers::make_config;
use super::*;
use std::collections::HashMap;

#[test]
fn test_fj202_when_true_includes_resource() {
    let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n    when: \"true\"\n";
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let order = vec!["pkg".to_string()];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(p.to_create, 1, "when: true should include the resource");
}

#[test]
fn test_fj202_when_false_excludes_resource() {
    let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n    when: \"false\"\n";
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let order = vec!["pkg".to_string()];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(p.to_create, 0, "when: false should exclude the resource");
    assert_eq!(p.changes.len(), 0);
}

#[test]
fn test_fj202_when_arch_match() {
    let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\n    arch: x86_64\nresources:\n  pkg:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [cuda]\n    when: '{{machine.arch}} == \"x86_64\"'\n";
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let order = vec!["pkg".to_string()];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(
        p.to_create, 1,
        "when arch matches, resource should be included"
    );
}

#[test]
fn test_fj202_when_arch_mismatch() {
    let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\n    arch: aarch64\nresources:\n  pkg:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [cuda]\n    when: '{{machine.arch}} == \"x86_64\"'\n";
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let order = vec!["pkg".to_string()];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(
        p.to_create, 0,
        "when arch doesn't match, resource should be excluded"
    );
}

#[test]
fn test_fj202_when_param_condition() {
    let yaml = "\nversion: \"1.0\"\nname: test\nparams:\n  env: staging\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  debug-pkg:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [strace]\n    when: '{{params.env}} != \"production\"'\n";
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let order = vec!["debug-pkg".to_string()];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(p.to_create, 1, "env=staging != production, should include");
}

#[test]
fn test_fj202_when_param_condition_false() {
    let yaml = "\nversion: \"1.0\"\nname: test\nparams:\n  env: production\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  debug-pkg:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [strace]\n    when: '{{params.env}} != \"production\"'\n";
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let order = vec!["debug-pkg".to_string()];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(
        p.to_create, 0,
        "env=production != production is false, should exclude"
    );
}

#[test]
fn test_fj202_when_no_condition_applies() {
    // Resource without when: should always be included
    let config = make_config();
    let order = vec!["pkg".to_string(), "conf".to_string(), "svc".to_string()];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(
        p.to_create, 3,
        "resources without when: should all be included"
    );
}

#[test]
fn test_fj202_when_mixed_conditions() {
    let yaml = "\nversion: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  always:\n    type: file\n    machine: m1\n    path: /tmp/always.txt\n    content: \"always\"\n  never:\n    type: file\n    machine: m1\n    path: /tmp/never.txt\n    content: \"never\"\n    when: \"false\"\n  conditional:\n    type: file\n    machine: m1\n    path: /tmp/cond.txt\n    content: \"cond\"\n    when: \"true\"\n";
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let order = vec![
        "always".to_string(),
        "conditional".to_string(),
        "never".to_string(),
    ];
    let locks = HashMap::new();
    let p = plan(&config, &order, &locks, None);
    assert_eq!(
        p.to_create, 2,
        "only 'always' and 'conditional' should be included"
    );
}
