//! FJ-2402: WASM deployment — build config, size budgets, CDN targets.
//!
//! ```bash
//! cargo run --example wasm_deploy
//! ```

use forjar::core::types::{
    CachePolicy, CdnTarget, WasmBuildResult, WasmOptLevel, WasmSizeBudget,
};

fn main() {
    // Optimization levels
    println!("=== WASM Optimization Levels ===");
    for level in [
        WasmOptLevel::Fast,
        WasmOptLevel::Balanced,
        WasmOptLevel::MaxSpeed,
        WasmOptLevel::MinSize,
    ] {
        println!("  {}: {}", level, level.flag());
    }
    println!();

    // Size budget
    let budget = WasmSizeBudget::default();
    println!("=== Size Budget ===");
    println!("  Core:     {} KB", budget.core_kb);
    println!("  Widgets:  {} KB", budget.widgets_kb);
    println!("  Full app: {} KB", budget.full_app_kb);
    println!();

    // Size checks
    let sizes = [(80, true), (100, true), (120, false)];
    println!("=== Core Size Checks ===");
    for (kb, expected) in sizes {
        let ok = budget.check_core(kb * 1024);
        let status = if ok { "PASS" } else { "FAIL" };
        println!("  {kb} KB: [{status}] (expected: {expected})");
    }
    println!();

    // CDN targets
    println!("=== CDN Targets ===");
    let targets = vec![
        CdnTarget::S3 {
            bucket: "interactive.paiml.com-production".into(),
            region: Some("us-east-1".into()),
            distribution: Some("ELY820FVFXAFF".into()),
        },
        CdnTarget::Cloudflare {
            project: "presentar-app".into(),
        },
        CdnTarget::Local {
            path: "/var/www/html".into(),
        },
    ];
    for t in &targets {
        println!("  {}: {t}", t.name());
    }
    println!();

    // Cache policies
    println!("=== Cache Policies ===");
    for policy in CachePolicy::defaults() {
        println!(
            "  {} -> {} ({})",
            policy.extension, policy.cache_control, policy.ttl
        );
    }
    println!();

    // Build result
    let result = WasmBuildResult {
        wasm_path: "dist/pkg/presentar_bg.wasm".into(),
        wasm_size: 480 * 1024,
        js_size: 15 * 1024,
        total_size: 495 * 1024,
        duration_secs: 8.3,
        opt_level: WasmOptLevel::MinSize,
    };
    println!("=== Build Result ===");
    println!("  {result}");
    let within = budget.check_full_app(result.total_size);
    println!(
        "  Within budget: {} ({} KB / {} KB)",
        within,
        result.total_kb(),
        budget.full_app_kb
    );
}
