# Policy-as-Code Engine

Forjar's policy engine (FJ-3200) evaluates declarative rules against your
infrastructure configuration at plan time — before any changes reach machines.

## Policy Types

| Type | Behavior | Default Severity |
|------|----------|-----------------|
| `assert` | Field must equal expected value | error |
| `deny` | Field must NOT equal value | error |
| `require` | Field must be present | error |
| `limit` | List field count within bounds | warning |
| `warn` | Like deny but advisory | warning |

## Configuration

Add a `policies` section to your `forjar.yaml`:

```yaml
policies:
  # All config files must be owned by root
  - type: assert
    id: SEC-001
    message: "config files must be owned by root"
    resource_type: file
    tag: config
    condition_field: owner
    condition_value: root
    severity: error
    remediation: "Set owner: root on the resource"
    compliance:
      - framework: cis
        control: "6.1.2"

  # All files must have explicit permissions
  - type: require
    id: SEC-002
    message: "all files must have explicit mode"
    resource_type: file
    field: mode
    remediation: "Add mode field (e.g., mode: '0644')"

  # Package resources should be small
  - type: limit
    id: PERF-001
    message: "package resources should have fewer than 5 packages"
    resource_type: package
    field: packages
    max_count: 5
    severity: warning
    remediation: "Split into multiple package resources"

  # All resources should have tags
  - type: limit
    id: OPS-001
    message: "all resources must have at least 1 tag"
    field: tags
    min_count: 1
    severity: info
    remediation: "Add tags for selective filtering"
```

## Rule Fields

| Field | Required | Description |
|-------|----------|-------------|
| `type` | yes | `assert`, `deny`, `require`, `limit`, `warn` |
| `message` | yes | Human-readable violation message |
| `id` | no | Policy ID (e.g., `SEC-001`) for tracking |
| `resource_type` | no | Scope to resource type (`file`, `package`, etc.) |
| `tag` | no | Scope to resources with this tag |
| `field` | conditional | Target field for `require` and `limit` |
| `condition_field` | conditional | Field to check for `assert`, `deny`, `warn` |
| `condition_value` | conditional | Expected value for condition checks |
| `max_count` | no | Maximum items for `limit` type |
| `min_count` | no | Minimum items for `limit` type |
| `severity` | no | Override: `error`, `warning`, or `info` |
| `remediation` | no | Fix suggestion shown on violation |
| `compliance` | no | List of `{framework, control}` mappings |

## Severity Levels

- **error** — Blocks `forjar apply`. Must be fixed before deployment.
- **warning** — Reported but does not block. Should be addressed.
- **info** — Advisory. Logged for visibility.

Each policy type has a default severity. The `severity` field overrides it.

## Running Policy Checks

```bash
# Text output
forjar policy forjar.yaml

# JSON output (for CI integration)
forjar policy forjar.yaml --json
```

Policy checks also run automatically during `forjar apply`. Error-severity
violations block the apply with a clear message and remediation hints.

## JSON Output

The `--json` flag produces structured output for CI pipelines:

```json
{
  "passed": false,
  "rules_evaluated": 4,
  "resources_checked": 3,
  "error_count": 1,
  "warning_count": 1,
  "info_count": 2,
  "violations": [
    {
      "policy_id": "SEC-002",
      "resource_id": "app-conf",
      "message": "all files must have explicit mode",
      "severity": "error",
      "rule_type": "require",
      "remediation": "Add mode field (e.g., mode: '0644')",
      "compliance": []
    }
  ]
}
```

## Compliance Mappings

Map policy rules to compliance frameworks for audit trails:

```yaml
compliance:
  - framework: cis
    control: "6.1.2"
  - framework: stig
    control: "V-238300"
  - framework: soc2
    control: "CC6.1"
```

Compliance data appears in both text and JSON output, making it easy to
generate audit reports showing which controls are enforced.

## Scope Filtering

Rules can be scoped by resource type, tag, or both:

```yaml
# Only applies to file resources
- type: require
  resource_type: file
  field: mode

# Only applies to resources tagged "production"
- type: assert
  tag: production
  condition_field: owner
  condition_value: root

# Both: file resources tagged "config"
- type: assert
  resource_type: file
  tag: config
  condition_field: mode
  condition_value: "0644"
```

Rules without scope filters apply to all resources.

## Example

Run the built-in example to see the policy engine in action:

```bash
cargo run --example policy_engine
```

This demonstrates all 5 policy types with compliance mappings, severity
overrides, and JSON output.
