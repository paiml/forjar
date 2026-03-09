//! Example: Plugin hot-reload via BLAKE3 hash check (FJ-3406)
//!
//! Demonstrates plugin caching, change detection, and reload
//! when .wasm files are modified on disk.
//!
//! ```bash
//! cargo run --example plugin_hot_reload
//! ```

use forjar::core::plugin_hot_reload::{compute_file_hash, PluginCache, ReloadCheck};
use forjar::core::types::PluginManifest;
fn main() {
    println!("=== Plugin Hot-Reload via BLAKE3 (FJ-3406) ===\n");

    let dir = tempfile::tempdir().unwrap();

    // Create two simulated plugin WASM files
    let alpha_path = dir.path().join("alpha.wasm");
    let beta_path = dir.path().join("beta.wasm");
    std::fs::write(&alpha_path, b"alpha wasm module v1").unwrap();
    std::fs::write(&beta_path, b"beta wasm module v1").unwrap();

    let mut cache = PluginCache::new();
    println!("1. Initial State:");
    println!("   Cache empty: {}", cache.is_empty());
    println!("   Generation: {}\n", cache.generation());

    // Load plugins into cache
    cache.insert("alpha", manifest("alpha"), alpha_path.clone());
    cache.insert("beta", manifest("beta"), beta_path.clone());
    println!("2. After Loading:");
    println!("   Cached plugins: {}", cache.len());
    println!("   Generation: {}", cache.generation());

    // Check reload status (should be up-to-date)
    println!("\n3. Reload Checks (before modification):");
    print_reload("alpha", &cache);
    print_reload("beta", &cache);

    // Simulate plugin update
    std::fs::write(&alpha_path, b"alpha wasm module v2 - updated").unwrap();
    println!("\n4. After modifying alpha.wasm:");
    print_reload("alpha", &cache);
    print_reload("beta", &cache);

    // Show stale plugins
    let stale = cache.stale_plugins();
    println!("\n5. Stale Plugins: {:?}", stale);

    // Hash computation
    println!("\n6. BLAKE3 Hashes:");
    if let Some(hash) = compute_file_hash(&alpha_path) {
        println!("   alpha.wasm: {}", &hash[..16]);
    }
    if let Some(hash) = compute_file_hash(&beta_path) {
        println!("   beta.wasm:  {}", &hash[..16]);
    }

    // Reload the changed plugin
    cache.insert("alpha", manifest("alpha"), alpha_path);
    println!("\n7. After reload:");
    print_reload("alpha", &cache);
    println!("   Stale: {:?}", cache.stale_plugins());

    println!("\nDone.");
}

fn manifest(name: &str) -> PluginManifest {
    PluginManifest {
        name: name.to_string(),
        version: "0.1.0".into(),
        description: Some(format!("{name} plugin")),
        abi_version: 1,
        wasm: format!("{name}.wasm"),
        blake3: String::new(),
        permissions: Default::default(),
        schema: None,
    }
}

fn print_reload(name: &str, cache: &PluginCache) {
    let check = cache.needs_reload(name);
    let status = match &check {
        ReloadCheck::UpToDate => "up-to-date".to_string(),
        ReloadCheck::Changed { old_hash, new_hash } => {
            format!("CHANGED ({}.. → {}..)", &old_hash[..8], &new_hash[..8])
        }
        ReloadCheck::NotCached => "not cached".to_string(),
        ReloadCheck::FileGone => "file gone!".to_string(),
    };
    println!("   {name:<10} → {status}");
}
