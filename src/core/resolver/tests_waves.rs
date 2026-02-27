use super::dag::compute_parallel_waves;
use super::*;

#[test]
fn test_fj216_waves_no_deps() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  a:
    type: file
    machine: m1
    path: /a
  b:
    type: file
    machine: m1
    path: /b
  c:
    type: file
    machine: m1
    path: /c
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let waves = compute_parallel_waves(&config).unwrap();
    assert_eq!(waves.len(), 1, "all independent = single wave");
    assert_eq!(waves[0].len(), 3);
}

#[test]
fn test_fj216_waves_linear_chain() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  a:
    type: file
    machine: m1
    path: /a
  b:
    type: file
    machine: m1
    path: /b
    depends_on: [a]
  c:
    type: file
    machine: m1
    path: /c
    depends_on: [b]
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let waves = compute_parallel_waves(&config).unwrap();
    assert_eq!(waves.len(), 3, "linear chain = 3 waves");
    assert_eq!(waves[0], vec!["a"]);
    assert_eq!(waves[1], vec!["b"]);
    assert_eq!(waves[2], vec!["c"]);
}

#[test]
fn test_fj216_waves_diamond() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  root:
    type: file
    machine: m1
    path: /root
  left:
    type: file
    machine: m1
    path: /left
    depends_on: [root]
  right:
    type: file
    machine: m1
    path: /right
    depends_on: [root]
  join:
    type: file
    machine: m1
    path: /join
    depends_on: [left, right]
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let waves = compute_parallel_waves(&config).unwrap();
    assert_eq!(waves.len(), 3, "diamond = 3 waves");
    assert_eq!(waves[0], vec!["root"]);
    assert_eq!(waves[1], vec!["left", "right"]); // concurrent
    assert_eq!(waves[2], vec!["join"]);
}

#[test]
fn test_fj216_waves_mixed() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  base:
    type: file
    machine: m1
    path: /base
  pkg-a:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    depends_on: [base]
  pkg-b:
    type: package
    machine: m1
    provider: apt
    packages: [git]
    depends_on: [base]
  independent:
    type: file
    machine: m1
    path: /ind
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let waves = compute_parallel_waves(&config).unwrap();
    assert_eq!(waves.len(), 2);
    // Wave 0: base + independent (both have in-degree 0)
    assert!(waves[0].contains(&"base".to_string()));
    assert!(waves[0].contains(&"independent".to_string()));
    // Wave 1: pkg-a + pkg-b (both depend only on base)
    assert!(waves[1].contains(&"pkg-a".to_string()));
    assert!(waves[1].contains(&"pkg-b".to_string()));
}
