# Platform Features

Forjar's platform capabilities that bridge the gap between "good IaC tool" and "world-class platform."

## Container Image Builds (FJ-2101–FJ-2106)

Forjar builds OCI-compliant container images from declarative YAML without requiring a Docker daemon.

### Three Build Paths

| Path | When to Use | Mechanism |
|------|-------------|-----------|
| **Direct tar** | File-only layers | Construct tar layers directly from resource content |
| **Pepita-to-OCI** | Build layers | Run commands in pepita namespace, export overlay diff |
| **Container sandbox** | Full Dockerfile-like builds | Run in Docker/Podman container, export result |

### Building an Image

```yaml
resources:
  my-app:
    type: image
    name: myapp
    version: "1.0.0"
    path: /opt/app/bin/server
    command: "/opt/app/bin/server"
    content: |
      #!/bin/sh
      exec /opt/app/bin/server
```

```bash
# Build the image
forjar build -f config.yaml --resource my-app

# Build and load into local Docker
forjar build -f config.yaml --resource my-app --load

# Build and push to registry
forjar build -f config.yaml --resource my-app --push

# Build and archive as FAR
forjar build -f config.yaml --resource my-app --far
```

### Build Caching (E16)

Forjar computes a BLAKE3 hash over all layer inputs (file paths,
content, permissions). On subsequent builds, if the input hash
matches the cached value, the rebuild is skipped entirely.

```
$ forjar build -f config.yaml --resource my-app
Building my-app (myapp:1.0.0) — CACHED
  Layer inputs unchanged (hash: a1b2c3d4e5f6...), skipping rebuild
```

### Build Metrics (E17)

Every build writes `build-metrics.json` to the output directory:

```json
{
  "tag": "myapp:1.0.0",
  "layer_count": 2,
  "total_size": 1048576,
  "layers": [
    { "file_count": 3, "uncompressed_size": 2000000, "compressed_size": 800000 }
  ],
  "duration_secs": 1.5,
  "built_at": "2026-03-07T22:00:00Z",
  "forjar_version": "1.1.1",
  "target_arch": "x86_64"
}
```

### Parallel Layer Building (E18)

When an image has multiple layers, forjar builds them concurrently
using `std::thread::scope`. Each layer's tar creation and gzip
compression runs in its own thread. Single-layer images skip thread
overhead. This scales linearly with layer count on multi-core machines.

### Automatic Layer Splitting (E13)

Forjar automatically separates config files from application binaries
into distinct OCI layers. Config files (`.yaml`, `.toml`, `.json`,
`.conf`, `.cfg`, `.ini`, `.env`, `.properties`) go to a top layer
that changes frequently, while binaries go to a lower layer that
changes rarely. This improves registry push efficiency — only the
changed layer needs uploading.

### Registry Push (FJ-2105)

Implements OCI Distribution v1.1 protocol:
1. Discover blobs in OCI layout
2. Check existing blobs on registry (HEAD request)
3. Upload missing blobs — monolithic PUT for < 64 MB, chunked
   PATCH + PUT for >= 64 MB (E14)
4. Upload manifest

**Chunked uploads (E14)**: Large layers use OCI chunked upload
protocol — 16 MB PATCH chunks with `Content-Range` headers,
following `Location` header between chunks, finalized with PUT
including the blob digest.

## Image Drift Detection (E15)

Forjar extends drift detection to deployed container images.
When a resource of type `image` is converged, forjar stores the
manifest digest. On `forjar drift`, it runs `docker inspect` on
the target machine and compares the actual image digest to the
expected one.

```
$ forjar drift -f config.yaml
Checking builder (3 resources)...
  app-server: OK
  web-frontend: DRIFTED — deployed image differs from built image
  worker: DRIFTED — container not running
```

Drift scenarios detected:
- **Digest mismatch**: someone pushed a different image to the container
- **Container not running**: expected container has stopped
- **Transport error**: machine unreachable

## Task Input Caching (FJ-2701)

Task resources with `cache: true` and `task_inputs` patterns use
BLAKE3 hashing to skip re-execution when inputs haven't changed.

```yaml
resources:
  build-app:
    type: task
    cache: true
    task_inputs:
      - "src/**/*.rs"
      - Cargo.toml
    command: "cargo build --release"
```

On apply, forjar hashes all files matching the input patterns. If
the hash matches the stored value from the last successful run, the
task is skipped. The input hash is persisted in the state lock's
resource details.

## Machine Connectivity Probing (E19)

Active transport probing verifies machine reachability before applying changes.

```bash
forjar status --connectivity -f config.yaml
```

Output:
```
  ● web-server [ssh] reachable (45ms)
  ● db-server [ssh] reachable (52ms)
  ● local-dev [local] reachable (0ms)
  ● staging [container] unreachable — container not running

Connectivity: 3/4 machines reachable
```

### Transport Probes

| Transport | Probe Command | Timeout |
|-----------|--------------|---------|
| SSH | `ssh -o ConnectTimeout=5 -o BatchMode=yes user@addr true` | 5s |
| Container | `docker exec <name> true` (or podman) | Default |
| Local | Always reachable | 0ms |

JSON output with `--json`:
```json
[
  {
    "machine": "web-server",
    "transport": "ssh",
    "reachable": true,
    "latency_ms": 45,
    "error": null
  }
]
```

## Structured Run Logs (E20)

Run output is captured in dual format:
- `.log` — Human-readable with section delimiters
- `.json` — Machine-parseable structured JSON

```
state/<machine>/runs/<run-id>/
  meta.yaml          # Run metadata (YAML)
  meta.json          # Run metadata (JSON)
  <resource>.apply.log   # Human-readable log
  <resource>.apply.json  # Structured JSON log
  <resource>.script      # Raw script
```

The JSON log includes: resource_id, resource_type, action,
machine, transport, script, script_hash (BLAKE3), stdout, stderr,
exit_code, duration_secs, timestamps.

## Task Mode Script Generation (E21)

Each `task_mode` generates a distinct shell script optimized for its execution pattern.

### Batch Mode (Default)

Direct command execution:
```bash
set -euo pipefail
cargo build --release
```

### Pipeline Mode

Sequential stages with optional gate enforcement:
```bash
set -euo pipefail
FORJAR_PIPELINE_OK=0
echo '=== Stage: lint ==='
if ! bash -c 'cargo clippy -- -D warnings'; then
  echo 'GATE FAILED: lint'
  exit 1
fi
echo '=== Stage: test ==='
cargo test
```

### Service Mode

Background process with PID file management and health checks:
```bash
set -euo pipefail
# Check if already running
if [ -f '/tmp/forjar-svc-api-server.pid' ] && \
   kill -0 "$(cat '/tmp/forjar-svc-api-server.pid')" 2>/dev/null; then
  echo 'service=api-server already running'
  exit 0
fi
# Start in background
nohup bash -c 'app serve --port 8080' > /tmp/forjar-svc-api-server.log 2>&1 &
echo $! > '/tmp/forjar-svc-api-server.pid'
# Health check with retry
for i in 1 2 3; do
  sleep 2
  if curl -sf http://localhost:8080/health; then exit 0; fi
done
```

### Dispatch Mode

Pre-flight quality gate before execution:
```bash
set -euo pipefail
# Pre-flight gate check
if ! bash -c 'test -f /opt/deploy-ready.flag'; then
  echo 'DISPATCH BLOCKED: gate check failed'
  exit 1
fi
# Execute dispatch command
deploy.sh v1.2.3 production
```

## Testable CLI Output (FJ-2920)

Forjar CLI commands use the `OutputWriter` trait to separate data output (stdout) from
status messages (stderr). This enables:

- **Unit testing** — inject a `TestWriter` to capture and assert output
- **Benchmarking** — inject a `NullWriter` to measure pure logic cost
- **Production** — `StdoutWriter` sends results to stdout, status/warnings to stderr

### The OutputWriter Trait

```rust
trait OutputWriter {
    fn status(&mut self, msg: &str);   // progress → stderr
    fn result(&mut self, msg: &str);   // data → stdout
    fn warning(&mut self, msg: &str);  // ⚠ → stderr
    fn error(&mut self, msg: &str);    // ✗ → stderr
    fn success(&mut self, msg: &str);  // ✓ → stderr
    fn flush(&mut self);
}
```

### Testing with TestWriter

```rust
let mut w = TestWriter::new();
cmd_lint_with_writer(&file, true, false, false, &mut w).unwrap();
assert!(w.stdout_text().contains("\"warnings\""));
assert!(w.stderr.is_empty()); // JSON mode: all output to result
```

### Pipeline-Safe Output

Because `result()` goes to stdout and `status()`/`warning()` go to stderr,
forjar commands are pipe-safe:

```bash
# Only data on stdout — warnings/progress stay on stderr
forjar lint --json config.yaml | jq '.findings'
forjar score --json config.yaml | jq '.grade'
```
