# Compliance Packs

Forjar's compliance pack system (FJ-3205) bundles related policy checks
into reusable, shareable packages aligned to industry frameworks like CIS,
STIG, and SOC2. A compliance pack is a single YAML file containing a set
of typed checks that evaluate the current state of a Forjar-managed
system and report pass/fail results.

## Compliance Pack YAML Format

A compliance pack is a YAML file with a metadata header and a list of
checks:

```yaml
pack:
  name: cis-linux-level1
  version: "1.0.0"
  framework: CIS
  description: "CIS Benchmark for Linux - Level 1"
  author: security-team
  tags: [linux, hardening, level1]

checks:
  - id: CIS-1.1.1
    description: "Ensure mounting of cramfs is disabled"
    type: deny
    resource_type: file
    match:
      path: /etc/modprobe.d/cramfs.conf
    condition:
      contains: "install cramfs /bin/true"

  - id: CIS-1.4.1
    description: "Ensure AIDE is installed"
    type: require
    resource_type: package
    match:
      packages: [aide]

  - id: CIS-5.2.1
    description: "Ensure SSH root login is disabled"
    type: assert
    resource_type: file
    match:
      path: /etc/ssh/sshd_config
    condition:
      line_matches: "^PermitRootLogin\\s+no"

  - id: CIS-CUSTOM-1
    description: "Verify firewall rules are active"
    type: script
    command: "iptables -L -n | grep -q 'Chain INPUT'"
    timeout: 10s
```

### Pack Metadata Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Unique identifier for the pack |
| `version` | string | yes | Semantic version of the pack |
| `framework` | string | no | Framework this pack implements (CIS, STIG, SOC2, etc.) |
| `description` | string | no | Human-readable description |
| `author` | string | no | Author or team name |
| `tags` | list | no | Tags for discovery and filtering |

## Check Types

Each check in a compliance pack has a `type` that determines how it is
evaluated.

### `assert`

Verifies that a resource exists and matches a condition. The check passes
when the condition evaluates to true against the matched resource.

```yaml
- id: SSH-001
  description: "SSH config disables password auth"
  type: assert
  resource_type: file
  match:
    path: /etc/ssh/sshd_config
  condition:
    line_matches: "^PasswordAuthentication\\s+no"
```

Supported conditions for `assert`:

| Condition | Description |
|-----------|-------------|
| `contains` | File content contains the given string |
| `line_matches` | At least one line matches the regex pattern |
| `equals` | Resource attribute equals the expected value |
| `min_value` | Numeric attribute is at least this value |
| `max_value` | Numeric attribute is at most this value |

### `deny`

The inverse of `assert`. The check passes when the matched resource does
**not** satisfy the condition. Use this to ensure dangerous configurations
are absent.

```yaml
- id: SEC-001
  description: "Ensure .rhosts files do not exist"
  type: deny
  resource_type: file
  match:
    path: /root/.rhosts
  condition:
    exists: true
```

### `require`

Verifies that a resource of the given type exists in the current state.
No condition is needed -- the check passes simply when the resource is
present.

```yaml
- id: PKG-001
  description: "Ensure auditd is installed"
  type: require
  resource_type: package
  match:
    packages: [auditd]
```

### `require_tag`

Verifies that all resources in the current state carry a specific tag.
This is useful for ensuring every resource has been classified or labelled
for audit purposes.

```yaml
- id: TAG-001
  description: "All resources must have an 'owner' tag"
  type: require_tag
  tag: owner
```

An optional `resource_type` field limits the check to resources of that
type:

```yaml
- id: TAG-002
  description: "All file resources must have a 'sensitivity' tag"
  type: require_tag
  tag: sensitivity
  resource_type: file
```

### `script`

Runs an arbitrary shell command on the controller. The check passes when
the command exits 0. Use this for checks that cannot be expressed
declaratively.

```yaml
- id: SCRIPT-001
  description: "Verify NTP synchronisation"
  type: script
  command: "chronyc tracking | grep -q 'Leap status.*Normal'"
  timeout: 10s
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | string | required | Shell command to execute |
| `timeout` | duration | `30s` | Maximum time for the command to run |

## Framework Mapping

Compliance packs map to industry frameworks through the `framework` field
and individual check IDs. Forjar recognises the following frameworks:

| Framework | Description | Typical Pack Prefix |
|-----------|-------------|---------------------|
| CIS | Center for Internet Security Benchmarks | `cis-` |
| STIG | Security Technical Implementation Guides (DoD) | `stig-` |
| SOC2 | Service Organization Control Type 2 | `soc2-` |
| HIPAA | Health Insurance Portability and Accountability Act | `hipaa-` |
| PCI-DSS | Payment Card Industry Data Security Standard | `pci-` |
| Custom | Organisation-specific policies | any |

The framework field is informational and used for reporting and
filtering. Check IDs should follow the naming convention of their
framework (e.g. `CIS-1.1.1`, `STIG-V-12345`, `SOC2-CC6.1`).

## Pack Evaluation

### `forjar comply eval`

Evaluate one or more compliance packs against the current state:

```bash
# Evaluate a single pack
forjar comply eval -p cis-linux-level1.yaml

# Evaluate multiple packs
forjar comply eval -p cis-linux-level1.yaml -p soc2-access.yaml

# Evaluate all packs in a directory
forjar comply eval --pack-dir ./compliance/

# Target a specific environment
forjar comply eval -p cis-linux-level1.yaml --env prod
```

### Output

```
Evaluating compliance pack: cis-linux-level1 v1.0.0 (CIS)
------------------------------------------------------------
  [PASS] CIS-1.1.1  Ensure mounting of cramfs is disabled
  [PASS] CIS-1.4.1  Ensure AIDE is installed
  [FAIL] CIS-5.2.1  Ensure SSH root login is disabled
         → line_matches: no line matching "^PermitRootLogin\s+no" in /etc/ssh/sshd_config
  [PASS] CIS-CUSTOM-1  Verify firewall rules are active

Results: 3 passed, 1 failed, 0 skipped
Compliance: FAILED
```

The command exits with a non-zero status if any check fails.

### Flags

| Flag | Default | Description |
|------|---------|-------------|
| `-p`, `--pack` | none | Path to a compliance pack YAML file (repeatable) |
| `--pack-dir` | none | Directory containing compliance pack files |
| `--env` | none | Evaluate against a specific environment's state |
| `-f`, `--file` | `forjar.yaml` | Path to Forjar config file |
| `--framework` | none | Only evaluate packs matching this framework |
| `--tag` | none | Only evaluate packs with this tag (repeatable) |
| `--json` | `false` | Output results as JSON |
| `--fail-on-skip` | `false` | Treat skipped checks as failures |

### JSON Output

```bash
forjar comply eval -p cis-linux-level1.yaml --json
```

```json
{
  "pack": "cis-linux-level1",
  "version": "1.0.0",
  "framework": "CIS",
  "total": 4,
  "passed": 3,
  "failed": 1,
  "skipped": 0,
  "compliant": false,
  "checks": [
    {
      "id": "CIS-1.1.1",
      "description": "Ensure mounting of cramfs is disabled",
      "type": "deny",
      "result": "pass"
    },
    {
      "id": "CIS-5.2.1",
      "description": "Ensure SSH root login is disabled",
      "type": "assert",
      "result": "fail",
      "detail": "line_matches: no line matching \"^PermitRootLogin\\s+no\" in /etc/ssh/sshd_config"
    }
  ]
}
```

## Pack Discovery

Forjar discovers compliance packs from directories using the `--pack-dir`
flag. Any file matching `*.yaml` or `*.yml` that contains a top-level
`pack:` key is treated as a compliance pack.

### Directory layout

```
compliance/
├── cis-linux-level1.yaml
├── cis-linux-level2.yaml
├── stig-rhel8.yaml
├── soc2-access.yaml
└── internal/
    ├── tagging-policy.yaml
    └── naming-conventions.yaml
```

Subdirectories are scanned recursively:

```bash
forjar comply eval --pack-dir ./compliance/
```

```
Discovered 6 compliance pack(s) in ./compliance/
  cis-linux-level1 v1.0.0 (CIS)
  cis-linux-level2 v1.0.0 (CIS)
  stig-rhel8 v2.1.0 (STIG)
  soc2-access v1.2.0 (SOC2)
  tagging-policy v1.0.0 (Custom)
  naming-conventions v1.0.0 (Custom)

Evaluating 6 pack(s)...
```

### Filtering discovered packs

Use `--framework` or `--tag` to evaluate a subset:

```bash
# Only CIS packs
forjar comply eval --pack-dir ./compliance/ --framework CIS

# Only packs tagged "hardening"
forjar comply eval --pack-dir ./compliance/ --tag hardening
```

### `forjar comply list`

List discovered packs without evaluating them:

```bash
forjar comply list --pack-dir ./compliance/
```

```
Pack                   Version  Framework  Checks  Tags
-----                  -------  ---------  ------  ----
cis-linux-level1       1.0.0    CIS        42      linux, hardening, level1
cis-linux-level2       1.0.0    CIS        18      linux, hardening, level2
stig-rhel8             2.1.0    STIG       156     rhel, hardening
soc2-access            1.2.0    SOC2       12      access-control
tagging-policy         1.0.0    Custom     3       governance
naming-conventions     1.0.0    Custom     5       governance
```

## CI Integration

Use compliance packs as a gate in CI pipelines:

```bash
# Fail the pipeline if any CIS check fails
forjar comply eval --pack-dir ./compliance/ --framework CIS --json \
  | jq -e '.[] | select(.compliant == false) | empty' \
  || { echo "CIS compliance check failed"; exit 1; }
```

Combine with promotion gates to enforce compliance before deploying to
production:

```yaml
environments:
  prod:
    promotion:
      from: staging
      gates:
        - validate: { deep: true }
        - policy: { strict: true }
        - script: "forjar comply eval --pack-dir ./compliance/ --framework CIS"
      rollout:
        strategy: canary
        canary_count: 1
```

## Writing Custom Packs

Create a compliance pack for organisation-specific policies:

```yaml
pack:
  name: acme-security
  version: "1.0.0"
  framework: Custom
  description: "ACME Corp internal security requirements"
  author: platform-team
  tags: [internal, security]

checks:
  - id: ACME-001
    description: "All services must run as non-root"
    type: deny
    resource_type: service
    match:
      user: root

  - id: ACME-002
    description: "All packages must have an owner tag"
    type: require_tag
    tag: owner
    resource_type: package

  - id: ACME-003
    description: "Firewall must be active"
    type: script
    command: "systemctl is-active firewalld || systemctl is-active ufw"
    timeout: 5s

  - id: ACME-004
    description: "Config files must not be world-writable"
    type: assert
    resource_type: file
    match:
      path: /etc/myapp/*
    condition:
      max_value: 644
```

## Summary of Check Types

| Type | Passes When | Use Case |
|------|-------------|----------|
| `assert` | Resource exists and condition is true | Verify correct configuration |
| `deny` | Resource does not exist or condition is false | Ensure dangerous config is absent |
| `require` | Resource of the given type exists | Verify required packages/services |
| `require_tag` | All matching resources carry the tag | Enforce governance labelling |
| `script` | Command exits 0 | Custom checks not expressible declaratively |
