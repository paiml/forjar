# WASM Resource Provider Plugins

Forjar's plugin system (FJ-3400) enables user-authored resource types
compiled to WebAssembly, running in a capability-based sandbox.

## Plugin Manifest

Every plugin has a `plugin.yaml` manifest:

```yaml
name: k8s-deployment
version: "0.1.0"
description: "Manage Kubernetes Deployments via kubectl"
abi_version: 1
wasm: k8s-deployment.wasm
blake3: "a7f3c2e1..."

permissions:
  fs:
    read: ["~/.kube/config"]
  net:
    connect: ["kubernetes.default.svc:443"]
  exec:
    allow: ["kubectl"]
  env:
    read: ["KUBECONFIG"]

schema:
  required: [name, namespace, image]
  properties:
    name: { type: string }
    namespace: { type: string }
    image: { type: string }
    replicas: { type: integer }
```

## Using Plugins in forjar.yaml

Reference plugins with the `plugin:` prefix in the resource type:

```yaml
resources:
  my-app:
    type: "plugin:k8s-deployment"
    machine: k8s-control-plane
    name: my-app
    namespace: production
    image: "registry.internal/my-app:v2.1"
    replicas: 3
```

## Capability-Based Sandbox

Plugins run in a WASM sandbox with explicit permissions:

| Permission | Description | Example |
|-----------|-------------|---------|
| `fs.read` | Paths the plugin can read | `~/.kube/config` |
| `fs.write` | Paths the plugin can write | `/tmp/plugin-state` |
| `net.connect` | Allowed host:port connections | `k8s.svc:443` |
| `exec.allow` | Binaries the plugin can execute | `kubectl` |
| `env.read` | Environment variables to expose | `KUBECONFIG` |

Permissions follow the principle of least privilege. A plugin with no
permissions declared runs in a fully isolated sandbox.

## Content-Addressed Verification

Every plugin's WASM module is verified using BLAKE3 hashing:

1. The manifest declares the expected `blake3` hash
2. Before loading, forjar computes the actual hash of the `.wasm` file
3. If hashes don't match, the plugin is rejected (tamper detection)

```
WASM hash: c93be142cc7c93188490575eba0b983529e1ada7...
Verify (correct):  true
Verify (tampered): false
```

## Schema Validation

Plugin schemas validate resource properties at plan time:

- **required**: Fields that must be present
- **properties**: Type checking (`string`, `integer`, `boolean`, `array`)

Invalid resources are caught before apply with clear error messages:

```
Missing fields: 2 errors
  - missing required property: namespace
  - missing required property: image
Wrong type: 1 errors
  - property 'replicas' expected type 'integer', got String("three")
```

## Plugin ABI

The Plugin ABI v1 defines four functions:

| Function | Signature | Description |
|----------|-----------|-------------|
| `check` | `(state) -> Status` | Check if resource is in desired state |
| `apply` | `(state) -> ApplyOutcome` | Converge resource to desired state |
| `destroy` | `(state) -> ()` | Remove the resource |
| `state_query` | `(state) -> StateMap` | Export resource state |

State is exchanged as MessagePack (compact, schema-free).

## Example

```bash
cargo run --example plugin_manifest
```
