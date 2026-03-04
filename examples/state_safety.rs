//! State safety and disaster recovery demonstration.
//!
//! Shows forjar's state protection mechanisms: integrity verification,
//! reversibility classification, convergence budget, and TUI plan review.

fn main() {
    println!("=== Forjar State Safety Demo ===\n");

    demo_integrity_verification();
    demo_reversibility();
    demo_convergence_budget();
    demo_tui_plan_review();

    println!("\n=== State safety checks complete ===");
}

fn demo_integrity_verification() {
    println!("--- State Integrity Verification ---");

    let tmp = tempfile::tempdir().unwrap();
    let state_dir = tmp.path();

    // Create a lock file using the public constructor
    let lock = forjar::core::state::new_lock("web-01", "web-01.example.com");

    let lock_path = state_dir.join("web-01.lock.yaml");
    let yaml = serde_yaml_ng::to_string(&lock).unwrap();
    std::fs::write(&lock_path, &yaml).unwrap();

    // Write BLAKE3 sidecar
    let hash = blake3::hash(yaml.as_bytes());
    let b3_path = state_dir.join("web-01.lock.yaml.b3");
    std::fs::write(&b3_path, hash.to_hex().as_str()).unwrap();

    // Verify integrity
    let stored_hash = std::fs::read_to_string(&b3_path).unwrap();
    let actual_hash = blake3::hash(&std::fs::read(&lock_path).unwrap())
        .to_hex()
        .to_string();
    let intact = stored_hash == actual_hash;
    println!("  Lock file: web-01.lock.yaml");
    println!("  BLAKE3 hash: {}...", &stored_hash[..16]);
    println!("  Integrity: {}", if intact { "VERIFIED" } else { "TAMPERED" });

    // Simulate tampering
    std::fs::write(&lock_path, "tampered content").unwrap();
    let tampered_hash = blake3::hash(&std::fs::read(&lock_path).unwrap())
        .to_hex()
        .to_string();
    let intact_after = stored_hash == tampered_hash;
    println!(
        "  After tampering: {}",
        if intact_after {
            "VERIFIED"
        } else {
            "TAMPERED (detected!)"
        }
    );
    println!();
}

fn demo_reversibility() {
    println!("--- Reversibility Classification ---");

    let operations = vec![
        ("package install", "Reversible", "Can uninstall"),
        ("file write", "Partially reversible", "Backup exists"),
        ("database drop", "Irreversible", "Data permanently lost"),
        ("service restart", "Reversible", "Can restart again"),
        ("config update", "Partially reversible", "Previous version in lock"),
    ];

    for (op, class, reason) in operations {
        let marker = match class {
            "Reversible" => "+",
            "Partially reversible" => "~",
            _ => "!",
        };
        println!("  [{marker}] {op}: {class} -- {reason}");
    }
    println!();
}

fn demo_convergence_budget() {
    println!("--- Convergence Budget Enforcement ---");

    let budget = 3;
    let mut converged = false;

    for i in 1..=budget + 1 {
        if i > budget {
            println!("  Attempt {i}: BUDGET EXHAUSTED -- stopping retries");
            break;
        }
        let success = i == 3;
        if success {
            converged = true;
            println!("  Attempt {i}: CONVERGED");
            break;
        }
        println!("  Attempt {i}: FAILED -- retrying ({i}/{budget})");
    }
    println!(
        "  Result: {}",
        if converged {
            "Resource converged within budget"
        } else {
            "Budget exhausted"
        }
    );
    println!();
}

fn demo_tui_plan_review() {
    use forjar::cli::tui::*;

    println!("--- TUI Plan Review ---");

    let changes = vec![
        ("pkg-nginx".into(), "create".into(), "install nginx 1.24".into()),
        (
            "file-config".into(),
            "update".into(),
            "write /etc/nginx/nginx.conf".into(),
        ),
        (
            "svc-old".into(),
            "destroy".into(),
            "remove legacy service".into(),
        ),
    ];

    let items = plan_to_tui_items(&changes);
    let state = TuiState::new("Apply Plan Review", items);
    let result = build_result(&state);

    println!("  Resources in plan: {}", state.items.len());
    println!("  Auto-approved: {} (create/update)", result.approved.len());
    println!(
        "  Requires confirmation: {} (destroy)",
        result.rejected.len()
    );

    for item in &state.items {
        let check = if item.selected { "[x]" } else { "[ ]" };
        println!(
            "    {check} {} -- {} ({})",
            item.id, item.description, item.action
        );
    }
    println!();
}
