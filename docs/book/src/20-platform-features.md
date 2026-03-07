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

### Registry Push (FJ-2105)

Implements OCI Distribution v1.1 protocol:
1. Discover blobs in OCI layout
2. Check existing blobs on registry (HEAD request)
3. Upload missing blobs (PUT)
4. Upload manifest

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
