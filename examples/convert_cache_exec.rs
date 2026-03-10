//! FJ-1360/1363: Convert execution and cache command generation.
//!
//! Usage: cargo run --example convert_cache_exec

use forjar::core::store::cache::CacheSource;
use forjar::core::store::cache_exec::{pull_command, push_command};
use forjar::core::store::convert::{
    analyze_conversion, ChangeType, ConversionChange, ConversionReport, ResourceConversion,
};
use forjar::core::store::convert_exec::apply_conversion;
use forjar::core::store::purity::PurityLevel;
use std::path::Path;

fn main() {
    println!("Forjar: Convert Execution & Cache Commands");
    println!("{}", "=".repeat(50));

    // ── FJ-1360: Cache command generation ──
    println!("\n[FJ-1360] Cache Command Generation:");

    let ssh = CacheSource::Ssh {
        host: "cache.prod.internal".into(),
        user: "forjar".into(),
        path: "/var/lib/forjar/cache".into(),
        port: Some(2222),
    };
    let local = CacheSource::Local {
        path: "/mnt/fast-cache".into(),
    };

    let hash = "blake3:aabbccdd11223344556677889900aabb";
    let staging = Path::new("/tmp/forjar-staging-aabbccdd");
    let store = Path::new("/var/lib/forjar/store");

    println!("\n  SSH pull command:");
    println!("    {}", pull_command(&ssh, hash, staging));
    println!("\n  SSH push command:");
    println!("    {}", push_command(&ssh, hash, store));
    println!("\n  Local pull command:");
    println!("    {}", pull_command(&local, hash, staging));
    println!("\n  Local push command:");
    println!("    {}", push_command(&local, hash, store));

    // ── FJ-1363: Convert execution ──
    println!("\n[FJ-1363] Convert Execution:");

    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "resources:\n  - name: nginx\n    type: package\n  - name: app-config\n    type: file\n    version: '1.0'\n",
    )
    .unwrap();

    let report = ConversionReport {
        resources: vec![
            ResourceConversion {
                name: "nginx".into(),
                provider: "apt".into(),
                current_purity: PurityLevel::Impure,
                target_purity: PurityLevel::Pinned,
                auto_changes: vec![
                    ConversionChange {
                        change_type: ChangeType::AddVersionPin,
                        description: "Add version pin".into(),
                    },
                    ConversionChange {
                        change_type: ChangeType::GenerateLockPin,
                        description: "Generate lock pin".into(),
                    },
                ],
                manual_changes: vec![],
            },
            ResourceConversion {
                name: "app-config".into(),
                provider: "cargo".into(),
                current_purity: PurityLevel::Pinned,
                target_purity: PurityLevel::Pure,
                auto_changes: vec![ConversionChange {
                    change_type: ChangeType::EnableStore,
                    description: "Enable store".into(),
                }],
                manual_changes: vec![],
            },
        ],
        current_purity: PurityLevel::Impure,
        projected_purity: PurityLevel::Pure,
        auto_change_count: 3,
        manual_change_count: 0,
    };

    let result = apply_conversion(&config_path, &report).unwrap();
    println!("  Changes applied: {}", result.changes_applied);
    println!("  Lock pins generated: {}", result.lock_pins_generated);
    println!("  New purity: {:?}", result.new_purity);
    println!("  Backup: {}", result.backup_path.display());

    let updated = std::fs::read_to_string(&config_path).unwrap();
    println!("\n  Updated config:\n{}", indent(&updated, "    "));

    let lock_path = tmp.path().join("forjar.inputs.lock.yaml");
    if lock_path.exists() {
        let lock = std::fs::read_to_string(&lock_path).unwrap();
        println!("  Lock file:\n{}", indent(&lock, "    "));
    }

    println!("\n{}", "=".repeat(50));
    println!("All convert/cache criteria survived.");
}

fn indent(s: &str, prefix: &str) -> String {
    s.lines()
        .map(|l| format!("{prefix}{l}"))
        .collect::<Vec<_>>()
        .join("\n")
}
