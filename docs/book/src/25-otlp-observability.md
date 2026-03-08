# OTLP Observability

Forjar exports W3C-compatible trace spans to any OpenTelemetry Protocol (OTLP) collector.

## Quick Start

```bash
forjar apply -f forjar.yaml --telemetry-endpoint http://localhost:4318
```

Forjar will:
1. Generate trace spans during apply (one per resource)
2. Write them to `state/<machine>/trace.jsonl`
3. POST them as OTLP/HTTP JSON to the endpoint

## Trace Structure

Each apply run generates a trace hierarchy:

```
forjar:apply (root span)
├── apply:nginx-pkg    (resource span, 150ms, OK)
├── apply:nginx-conf   (resource span, 50ms, OK)
├── apply:app-service  (resource span, 3200ms, ERROR)
└── apply:firewall     (resource span, 0ms, noop)
```

### Span Attributes

Each resource span includes:

| Attribute | Description |
|-----------|-------------|
| `forjar.resource_type` | package, file, service, etc. |
| `forjar.machine` | Target machine name |
| `forjar.action` | create, update, delete, noop |
| `forjar.exit_code` | 0 (success) or non-zero (failure) |
| `forjar.logical_clock` | Lamport clock for causal ordering |
| `forjar.content_hash` | BLAKE3 hash after apply |

### Trace IDs

Trace IDs are deterministic — derived from the run ID via FNV-1a hash. This means the same apply run always produces the same trace ID, enabling reproducible debugging.

## Compatible Collectors

Any OTLP/HTTP collector that accepts JSON:

| Collector | Endpoint |
|-----------|----------|
| Jaeger | `http://localhost:4318` |
| Grafana Tempo | `http://localhost:4318` |
| Datadog Agent | `https://trace.agent.datadoghq.com` |
| OpenTelemetry Collector | `http://localhost:4318` |

Forjar auto-appends `/v1/traces` to the endpoint if not present.

## Local Traces

Even without `--telemetry-endpoint`, traces are always written to `state/<machine>/trace.jsonl`:

```bash
# View trace spans for a machine
cat state/intel/trace.jsonl | jq .

# Query via MCP
forjar mcp trace --machine intel
```

## Example

```bash
cargo run --example otlp_export
```

This demo shows the OTLP JSON payload structure with sample spans.
