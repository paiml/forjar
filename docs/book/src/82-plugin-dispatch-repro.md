# Plugin Dispatch & Reproducible Builds

Falsification coverage for FJ-3404 and FJ-095.

## Plugin Dispatch (FJ-3404)

`type: "plugin:NAME"` resources are resolved, verified, and dispatched:

```rust
use forjar::core::plugin_dispatch::*;

// Parse plugin type strings
assert_eq!(parse_plugin_type("plugin:nginx"), Some("nginx"));
assert_eq!(parse_plugin_type("package"), None);
assert!(is_plugin_type("plugin:foo"));

// Dispatch operations (check/apply/destroy)
let result = dispatch_check(plugin_dir, "nginx", &config);
assert!(result.success);
assert_eq!(result.operation, "check");

// List available plugins
let types = available_plugin_types(plugin_dir);
// Returns ["plugin:nginx", "plugin:monitoring", ...]
```

Plugins are verified via BLAKE3 hash before dispatch. The WASM runtime is currently a stub (Phase 2).

## Reproducible Builds (FJ-095)

Bit-for-bit reproducible binaries from source:

```rust
use forjar::core::repro_build::*;

// Check build environment
let env_checks = check_environment();
// Checks: SOURCE_DATE_EPOCH, CARGO_INCREMENTAL, STRIP_SETTING

// Check Cargo.toml profile
let profile_checks = check_cargo_profile(cargo_toml);
// Checks: codegen-units=1, lto, panic=abort

// Hash source and binary
let src_hash = hash_source_dir(src_path)?;
let bin_hash = hash_binary(bin_path)?;

// Generate full report
let report = generate_report(&src_hash, &bin_hash, &cargo_toml);
assert!(report.reproducible); // all checks passed
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_plugin_dispatch_repro.rs` | 21 | ~223 |
