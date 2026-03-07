//! FJ-2300/E19: Machine connectivity probing demonstration.
//!
//! Shows how forjar probes machine transports (local, SSH, container)
//! for reachability before applying changes.
//!
//! ```bash
//! cargo run --example connectivity_probe
//! ```

use forjar::core::types::ForjarConfig;

fn main() {
    // Parse a config with different transport types
    let yaml = r#"
version: "1.0"
name: connectivity-demo
machines:
  local-dev:
    hostname: dev
    addr: 127.0.0.1
  remote-web:
    hostname: web
    addr: 192.168.1.10
    transport: ssh
    user: deploy
  app-container:
    hostname: app
    addr: container
    transport: container
    container:
      runtime: docker
      name: my-app
resources:
  test:
    type: file
    machine: local-dev
    path: /tmp/test
    content: "hello"
"#;

    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    println!("=== Machine Connectivity Overview ===\n");
    println!("Config: {} (v{})", config.name, config.version);
    println!("Machines: {}\n", config.machines.len());

    for (name, machine) in &config.machines {
        let transport = machine.transport.as_deref().unwrap_or("local");
        let probe = match transport {
            "local" => "always reachable (no probe needed)".to_string(),
            "ssh" => format!(
                "ssh -o ConnectTimeout=5 -o BatchMode=yes {}@{} true",
                if machine.user.is_empty() {
                    "root"
                } else {
                    &machine.user
                },
                machine.addr
            ),
            "container" => {
                let container_name = machine
                    .container
                    .as_ref()
                    .and_then(|c| c.name.as_deref())
                    .unwrap_or(name);
                let runtime = machine
                    .container
                    .as_ref()
                    .map(|c| c.runtime.as_str())
                    .unwrap_or("docker");
                format!("{runtime} exec {container_name} true")
            }
            _ => format!("unknown transport: {transport}"),
        };

        println!("  {name}:");
        println!("    transport: {transport}");
        println!("    addr: {}", machine.addr);
        println!("    probe: {probe}");
        println!();
    }

    println!("=== CLI Usage ===\n");
    println!("  forjar status --connectivity -f config.yaml");
    println!("  forjar status --connectivity -f config.yaml --json");
}
