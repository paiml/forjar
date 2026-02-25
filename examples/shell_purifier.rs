//! FJ-036: Demonstrate bashrs shell purification pipeline.
//!
//! Shows validation, linting, and purification of generated shell scripts.

fn main() {
    println!("=== FJ-036: bashrs Shell Purification ===\n");

    // 1. Validate a clean script
    let clean = "#!/bin/bash\nset -euo pipefail\nmkdir -p /tmp/test\n";
    match forjar::core::purifier::validate_script(clean) {
        Ok(()) => println!("[PASS] Clean script validated"),
        Err(e) => println!("[FAIL] Clean script: {e}"),
    }

    // 2. Lint a script and show diagnostics
    let script = "#!/bin/bash\nset -euo pipefail\necho hello\n";
    let result = forjar::core::purifier::lint_script(script);
    println!(
        "\n[LINT] {} diagnostics for echo script",
        result.diagnostics.len()
    );
    for d in &result.diagnostics {
        println!("  [{:?}] {}: {}", d.severity, d.code, d.message);
    }

    // 3. Count errors only
    let errors = forjar::core::purifier::lint_error_count(script);
    println!("\n[ERRORS] {errors} error-level diagnostics");

    // 4. Purify a script through the full pipeline
    let raw = "echo hello world";
    match forjar::core::purifier::purify_script(raw) {
        Ok(purified) => println!("\n[PURIFIED]\n  Input:  {raw}\n  Output: {purified}"),
        Err(e) => println!("\n[PURIFY] Could not purify: {e}"),
    }

    // 5. Validate a generated codegen script
    let codegen_script = forjar::core::codegen::check_script(&forjar::core::types::Resource {
        resource_type: forjar::core::types::ResourceType::File,
        machine: forjar::core::types::MachineTarget::Single("m1".into()),
        path: Some("/etc/test.conf".into()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        content: Some("key=value".into()),
        source: None,
        target: None,
        owner: None,
        group: None,
        mode: None,
        name: None,
        enabled: None,
        restart_on: vec![],
        fs_type: None,
        options: None,
        uid: None,
        shell: None,
        home: None,
        groups: vec![],
        ssh_authorized_keys: vec![],
        system_user: false,
        schedule: None,
        command: None,
        image: None,
        ports: vec![],
        environment: vec![],
        volumes: vec![],
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
        recipe: None,
        inputs: std::collections::HashMap::new(),
        arch: vec![],
        tags: vec![],
    })
    .unwrap();

    match forjar::core::purifier::validate_script(&codegen_script) {
        Ok(()) => println!("\n[PASS] Generated file check script validated by bashrs"),
        Err(e) => println!("\n[WARN] Generated file check script: {e}"),
    }

    println!("\n=== bashrs integration complete ===");
}
