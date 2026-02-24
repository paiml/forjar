//! Demonstrate recipe loading, input validation, and expansion.
//!
//! Usage: cargo run --example recipe_expansion

use forjar::core::recipe;
use forjar::core::types::MachineTarget;
use std::collections::HashMap;

fn main() {
    let recipe_yaml = r#"
recipe:
  name: web-server
  version: "1.0"
  description: "Nginx web server with config"
  inputs:
    domain:
      type: string
      description: "Server domain name"
    port:
      type: int
      default: 80
      min: 1
      max: 65535
    log_level:
      type: enum
      choices: [error, warn, info, debug]
      default: warn

resources:
  nginx-pkg:
    type: package
    provider: apt
    packages: [nginx]

  site-config:
    type: file
    path: "/etc/nginx/sites-enabled/{{inputs.domain}}"
    content: |
      server {
        listen {{inputs.port}};
        server_name {{inputs.domain}};
        error_log /var/log/nginx/error.log {{inputs.log_level}};
      }
    mode: "0644"
    depends_on: [nginx-pkg]

  nginx-svc:
    type: service
    name: nginx
    state: running
    enabled: true
    restart_on: [site-config]
    depends_on: [site-config]
"#;

    // 1. Parse recipe
    let recipe_file = recipe::parse_recipe(recipe_yaml).expect("Recipe parse failed");
    let meta = &recipe_file.recipe;
    println!(
        "Recipe: {} v{}",
        meta.name,
        meta.version.as_deref().unwrap_or("unversioned")
    );
    println!("Inputs:");
    for (name, input) in &meta.inputs {
        let default = input
            .default
            .as_ref()
            .map(|v| format!(" (default: {:?})", v))
            .unwrap_or_default();
        println!("  {}: {}{}", name, input.input_type, default);
    }
    println!("Resources: {}", recipe_file.resources.len());

    // 2. Build provided inputs as HashMap<String, Value>
    let mut user_inputs: HashMap<String, serde_yaml_ng::Value> = HashMap::new();
    user_inputs.insert(
        "domain".to_string(),
        serde_yaml_ng::Value::String("example.com".to_string()),
    );
    user_inputs.insert(
        "port".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(443)),
    );
    // log_level uses default (warn)

    // 3. Validate inputs
    let resolved = recipe::validate_inputs(meta, &user_inputs).expect("Validation failed");
    println!("\nResolved inputs:");
    for (k, v) in &resolved {
        println!("  {} = {}", k, v);
    }

    // 4. Expand into namespaced resources
    let machine = MachineTarget::Single("prod-server".to_string());
    let expanded = recipe::expand_recipe(
        "web",
        &recipe_file,
        &machine,
        &user_inputs,
        &[], // no external depends_on
    )
    .expect("Expansion failed");

    println!("\nExpanded resources ({}):", expanded.len());
    for (id, resource) in &expanded {
        println!(
            "  {} [{}] machine={:?}",
            id, resource.resource_type, resource.machine
        );
        if !resource.depends_on.is_empty() {
            println!("    depends_on: {:?}", resource.depends_on);
        }
        if let Some(ref path) = resource.path {
            println!("    path: {}", path);
        }
    }
}
