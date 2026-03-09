//! FJ-3500: Environment promotion pipeline example.
//!
//! Demonstrates environment definitions, resolution (param/machine merging),
//! and cross-environment diff analysis.

fn main() {
    let yaml = r#"
version: "1.0"
name: my-app
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
  db:
    hostname: db-01
    addr: 10.0.0.2
params:
  log_level: debug
  replicas: 1
resources:
  nginx:
    type: package
    machine: web
    packages: [nginx]
  app-conf:
    type: file
    machine: web
    path: /etc/app.conf
    owner: root
    mode: "0644"
environments:
  dev:
    description: "Development"
    params:
      log_level: debug
      replicas: 1
    machines:
      web:
        addr: dev-web.internal
      db:
        addr: dev-db.internal
  staging:
    description: "Staging"
    params:
      log_level: info
      replicas: 2
    machines:
      web:
        addr: staging-web.internal
      db:
        addr: staging-db.internal
    promotion:
      from: dev
      auto_approve: true
      gates:
        - validate: { deep: true }
        - policy: { strict: true }
  prod:
    description: "Production"
    params:
      log_level: warn
      replicas: 4
    machines:
      web:
        addr: prod-web.internal
      db:
        addr: prod-db.internal
    promotion:
      from: staging
      auto_approve: false
      gates:
        - validate: { deep: true, exhaustive: true }
        - policy: { strict: true }
        - coverage: { min: 95 }
        - script: "curl -sf http://staging-web.internal/health"
      rollout:
        strategy: canary
        canary_count: 1
        health_check: "curl -sf http://{{ machine.addr }}:8080/health"
        percentage_steps: [25, 50, 100]
"#;

    let config: forjar::core::types::ForjarConfig =
        serde_yaml_ng::from_str(yaml).expect("valid YAML");

    use forjar::core::types::environment;

    println!("=== Environment Promotion Pipeline ===\n");
    println!("Config: {} ({})", config.name, config.version);
    println!("Environments: {}\n", config.environments.len());

    // List environments
    for (name, env) in &config.environments {
        let desc = env.description.as_deref().unwrap_or("-");
        let promo = env
            .promotion
            .as_ref()
            .map(|p| format!("from: {}", p.from))
            .unwrap_or_else(|| "base".to_string());
        println!("  [{name}] {desc} ({promo})");
        if let Some(ref p) = env.promotion {
            println!(
                "    gates: {}  auto_approve: {}",
                p.gates.len(),
                p.auto_approve
            );
        }
    }

    // Resolve dev environment
    println!("\n--- Dev Environment ---");
    let dev = &config.environments["dev"];
    let dev_params = environment::resolve_env_params(&config.params, dev);
    let dev_machines = environment::resolve_env_machines(&config.machines, dev);
    for (k, v) in &dev_params {
        println!("  param: {k} = {v:?}");
    }
    for (name, m) in &dev_machines {
        println!("  machine: {name} → {}", m.addr);
    }

    // Resolve prod environment
    println!("\n--- Prod Environment ---");
    let prod = &config.environments["prod"];
    let prod_params = environment::resolve_env_params(&config.params, prod);
    let prod_machines = environment::resolve_env_machines(&config.machines, prod);
    for (k, v) in &prod_params {
        println!("  param: {k} = {v:?}");
    }
    for (name, m) in &prod_machines {
        println!("  machine: {name} → {}", m.addr);
    }

    // Diff dev vs prod
    println!("\n--- Diff: dev → prod ---");
    let diff =
        environment::diff_environments("dev", dev, "prod", prod, &config.params, &config.machines);
    println!("Total differences: {}", diff.total_diffs());
    for pd in &diff.param_diffs {
        println!(
            "  param [{:}]: {:?} → {:?}",
            pd.key, pd.source_value, pd.target_value
        );
    }
    for md in &diff.machine_diffs {
        println!(
            "  machine [{:}]: {:?} → {:?}",
            md.machine, md.source_addr, md.target_addr
        );
    }

    // State directory isolation
    println!("\n--- State Directories ---");
    let base = std::path::Path::new(".forjar/state");
    for name in config.environments.keys() {
        let dir = environment::env_state_dir(base, name);
        println!("  {name}: {}", dir.display());
    }
}
