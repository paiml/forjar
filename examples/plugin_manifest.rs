//! FJ-3400/3403: WASM resource provider plugin manifest and lifecycle example.
//!
//! Demonstrates plugin manifest parsing, BLAKE3 hash verification,
//! ABI compatibility checking, schema validation, and the full
//! plugin lifecycle: scaffold, install, list, verify, and remove.

fn main() {
    use forjar::core::plugin_loader::{list_plugins, resolve_and_verify, resolve_manifest};
    use forjar::core::types::{PluginManifest, PluginSchema, SchemaProperty, PLUGIN_ABI_VERSION};

    println!("=== FJ-3400/3403: Plugin Manifest & Lifecycle ===\n");
    println!("Host ABI version: {PLUGIN_ABI_VERSION}\n");

    // 1. Parse a plugin manifest
    let yaml = r#"
name: k8s-deployment
version: "0.1.0"
description: "Manage Kubernetes Deployments via kubectl"
abi_version: 1
wasm: k8s-deployment.wasm
blake3: "placeholder"
permissions:
  fs:
    read: ["~/.kube/config"]
  net:
    connect: ["kubernetes.default.svc:443"]
  exec:
    allow: ["kubectl"]
  env:
    read: ["KUBECONFIG", "KUBE_CONTEXT"]
schema:
  required: [name, namespace, image]
  properties:
    name:
      type: string
    namespace:
      type: string
    image:
      type: string
    replicas:
      type: integer
"#;

    let manifest: PluginManifest = serde_yaml_ng::from_str(yaml).expect("valid manifest");
    println!("Plugin: {manifest}");
    println!("Resource type: {}", manifest.resource_type());
    println!("ABI compatible: {}", manifest.is_abi_compatible());
    println!("Permissions empty: {}", manifest.permissions.is_empty());
    println!("  fs.read: {:?}", manifest.permissions.fs.read);
    println!("  exec.allow: {:?}", manifest.permissions.exec.allow);
    println!("  env.read: {:?}", manifest.permissions.env.read);

    // 2. BLAKE3 hash verification
    println!("\n--- BLAKE3 Verification ---");
    let wasm_bytes = b"(module (func (export \"check\") (nop)))";
    let hash = blake3::hash(wasm_bytes).to_hex().to_string();
    println!("WASM hash: {hash}");

    let verified_manifest: PluginManifest = serde_yaml_ng::from_str(&format!(
        r#"
name: test-plugin
version: "0.1.0"
abi_version: 1
wasm: test.wasm
blake3: "{hash}"
"#
    ))
    .unwrap();
    println!(
        "Verify (correct): {}",
        verified_manifest.verify_hash(wasm_bytes)
    );
    println!(
        "Verify (tampered): {}",
        verified_manifest.verify_hash(b"tampered!")
    );

    // 3. Schema validation
    println!("\n--- Schema Validation ---");
    let schema = manifest.schema.as_ref().unwrap();

    // Valid resource
    let mut valid = indexmap::IndexMap::new();
    valid.insert("name".into(), serde_yaml_ng::Value::String("my-app".into()));
    valid.insert(
        "namespace".into(),
        serde_yaml_ng::Value::String("prod".into()),
    );
    valid.insert(
        "image".into(),
        serde_yaml_ng::Value::String("nginx:latest".into()),
    );
    valid.insert(
        "replicas".into(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(3)),
    );
    let errors = schema.validate(&valid);
    println!("Valid resource: {} errors", errors.len());

    // Missing required field
    let mut invalid = indexmap::IndexMap::new();
    invalid.insert("name".into(), serde_yaml_ng::Value::String("my-app".into()));
    let errors = schema.validate(&invalid);
    println!("Missing fields: {} errors", errors.len());
    for e in &errors {
        println!("  - {e}");
    }

    // Wrong type
    let mut wrong_type = valid.clone();
    wrong_type.insert(
        "replicas".into(),
        serde_yaml_ng::Value::String("three".into()),
    );
    let errors = schema.validate(&wrong_type);
    println!("Wrong type: {} errors", errors.len());
    for e in &errors {
        println!("  - {e}");
    }

    // 4. Build a schema programmatically
    println!("\n--- Programmatic Schema ---");
    let mut props = indexmap::IndexMap::new();
    props.insert(
        "port".into(),
        SchemaProperty {
            prop_type: Some("integer".into()),
            default: Some(serde_yaml_ng::Value::Number(8080.into())),
            items: None,
        },
    );
    let schema = PluginSchema {
        required: vec!["port".into()],
        properties: props,
    };
    let mut test_props = indexmap::IndexMap::new();
    test_props.insert("port".into(), serde_yaml_ng::Value::Number(9090.into()));
    println!("Schema valid: {}", schema.validate(&test_props).is_empty());

    // 5. Plugin lifecycle: scaffold, install, list, verify, remove
    println!("\n--- Plugin Lifecycle (install / list / verify / remove) ---");

    let tmpdir = tempfile::tempdir().expect("create temp dir");
    let staging = tmpdir.path().join("staging");
    let plugins = tmpdir.path().join("plugins");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::create_dir_all(&plugins).unwrap();

    // 5a. Scaffold a plugin in the staging area
    let plugin_name = "demo-plugin";
    let plugin_staging = staging.join(plugin_name);
    std::fs::create_dir_all(&plugin_staging).unwrap();
    let wasm_stub = b"(module)";
    let wasm_hash = blake3::hash(wasm_stub).to_hex().to_string();
    let manifest_yaml = format!(
        "name: {plugin_name}\n\
         version: \"0.2.0\"\n\
         abi_version: 1\n\
         wasm: plugin.wasm\n\
         blake3: \"{wasm_hash}\"\n\
         description: \"Demo plugin for lifecycle example\"\n"
    );
    std::fs::write(plugin_staging.join("plugin.yaml"), &manifest_yaml).unwrap();
    std::fs::write(plugin_staging.join("plugin.wasm"), wasm_stub).unwrap();
    println!("  Scaffolded '{plugin_name}' in staging");

    // 5b. Install: copy from staging to plugins directory
    let dest = plugins.join(plugin_name);
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::copy(plugin_staging.join("plugin.yaml"), dest.join("plugin.yaml")).unwrap();
    std::fs::copy(plugin_staging.join("plugin.wasm"), dest.join("plugin.wasm")).unwrap();
    println!("  Installed '{plugin_name}' to {}", plugins.display());

    // 5c. List installed plugins
    let installed = list_plugins(&plugins);
    println!("  Installed plugins: {:?}", installed);

    // 5d. Resolve and verify
    let resolved = resolve_and_verify(&plugins, plugin_name).unwrap();
    println!(
        "  Verified: {} v{} — status={:?}",
        resolved.manifest.name, resolved.manifest.version, resolved.status
    );

    // 5e. Resolve manifest only
    let m = resolve_manifest(&plugins, plugin_name).unwrap();
    println!(
        "  Manifest: {} v{}, ABI v{}, wasm={}",
        m.name, m.version, m.abi_version, m.wasm
    );

    // 5f. Remove the plugin
    std::fs::remove_dir_all(&dest).unwrap();
    let remaining = list_plugins(&plugins);
    println!("  Removed '{plugin_name}' — remaining: {:?}", remaining);

    println!("\nDone.");
}
