//! Example: Shell provider bridge (FJ-3405)
//!
//! Demonstrates validating shell scripts as resource providers,
//! including bashrs validation and secret leakage detection.
//!
//! ```bash
//! cargo run --example shell_provider
//! ```

use forjar::core::shell_provider;

fn main() {
    println!("=== Shell Provider Bridge (FJ-3405) ===\n");

    // Parse shell provider types
    println!("1. Type Parsing:");
    let types = ["shell:nginx", "shell:postgres", "plugin:foo", "file"];
    for t in &types {
        let is_shell = shell_provider::is_shell_type(t);
        let name = shell_provider::parse_shell_type(t);
        println!("  {t:<20} shell={is_shell:<5} name={name:?}");
    }

    // Validate clean scripts
    println!("\n2. Script Validation (clean):");
    let clean = "#!/bin/bash\nset -euo pipefail\nsystemctl restart nginx\n";
    match shell_provider::validate_provider_script(clean) {
        Ok(()) => println!("  Clean script: PASS"),
        Err(e) => println!("  Clean script: FAIL — {e}"),
    }

    // Validate leaky scripts
    println!("\n3. Script Validation (leaky):");
    let leaky_scripts = [
        ("echo $PASSWORD", "echo password leak"),
        ("curl -u admin:pass https://api.com", "curl inline creds"),
        ("sshpass -p secret ssh host", "sshpass inline"),
        ("export TOKEN=abc123", "export secret"),
    ];

    for (script, label) in &leaky_scripts {
        let full = format!("#!/bin/bash\n{script}\n");
        match shell_provider::validate_provider_script(&full) {
            Ok(()) => println!("  {label}: PASS (unexpected)"),
            Err(_) => println!("  {label}: BLOCKED (secret leakage detected)"),
        }
    }

    // Create and validate a full provider
    println!("\n4. Full Provider Validation:");
    let dir = tempfile::tempdir().unwrap();
    let pdir = dir.path().join("my-service");
    std::fs::create_dir_all(&pdir).unwrap();

    std::fs::write(
        pdir.join("provider.yaml"),
        "name: my-service\nversion: \"1.0.0\"\ndescription: \"Example service provider\"\ncheck: check.sh\napply: apply.sh\ndestroy: destroy.sh\n",
    ).unwrap();

    std::fs::write(
        pdir.join("check.sh"),
        "#!/bin/bash\nset -euo pipefail\nsystemctl is-active my-service\n",
    )
    .unwrap();
    std::fs::write(
        pdir.join("apply.sh"),
        "#!/bin/bash\nset -euo pipefail\nsystemctl start my-service\n",
    )
    .unwrap();
    std::fs::write(
        pdir.join("destroy.sh"),
        "#!/bin/bash\nset -euo pipefail\nsystemctl stop my-service\n",
    )
    .unwrap();

    let result = shell_provider::validate_provider(&pdir);
    println!("  Provider: {}", result.name);
    println!("  Validated: {}", result.validated);
    println!("  Status: {:?}", result.status);
    println!("  Errors: {}", result.errors.len());

    // List providers
    println!("\n5. List Providers:");
    let providers = shell_provider::list_shell_providers(dir.path());
    for p in &providers {
        println!("  - {p}");
    }

    println!("\nDone.");
}
