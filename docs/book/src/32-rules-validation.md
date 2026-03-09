# Rulebook Validation

Forjar's rulebook validation system (FJ-3108) provides static analysis of
event-driven rulebook configurations before deployment. The `forjar rules`
subcommands catch misconfigurations early -- duplicate names, empty event
lists, invalid actions, and cooldown anti-patterns -- so issues surface in
development or CI rather than production.

## `forjar rules validate`

The `validate` subcommand parses a rulebook YAML file and runs semantic
checks across all defined rulebooks:

```bash
forjar rules validate -f rulebooks.yaml
```

Output:

```
Validating rulebooks in rulebooks.yaml
------------------------------------------------------------
3 rulebook(s), 1 error(s), 1 warning(s)
  [ERROR] bad-notify: action[0] notify.channel is empty
  [WARN ] rapid-fire: cooldown_secs=0 may cause rapid-fire triggering

Validation FAILED.
```

The command exits with a non-zero status if any errors are found. Warnings
are advisory and do not cause failure.

### Flags

| Flag | Default | Description |
|------|---------|-------------|
| `-f`, `--file` | `forjar.yaml` | Path to rulebook YAML file |
| `--json` | `false` | Output results as JSON (for CI pipelines) |

## Semantic Checks

The validation engine performs the following checks on each rulebook:

### Duplicate names (error)

Every rulebook in a file must have a unique `name`. Duplicates are flagged
as errors because they cause ambiguity in event routing and cooldown tracking.

```yaml
rulebooks:
  - name: config-repair    # OK
    events: [{type: file_changed}]
    actions: [{script: "echo fix"}]
  - name: config-repair    # ERROR: duplicate rulebook name
    events: [{type: manual}]
    actions: [{script: "echo manual"}]
```

### Empty events (error)

A rulebook with no event patterns will never trigger. This is always an error.

```yaml
rulebooks:
  - name: never-fires
    events: []              # ERROR: no event patterns defined
    actions: [{script: "echo unreachable"}]
```

### Empty actions (error)

A rulebook with no actions has nothing to execute when triggered.

```yaml
rulebooks:
  - name: does-nothing
    events: [{type: manual}]
    actions: []             # ERROR: no actions defined
```

### Empty action entry (error)

An action entry must have exactly one action type configured (`apply`,
`destroy`, `script`, or `notify`). An entry with no type is an error.

```yaml
actions:
  - {}                      # ERROR: action[0] has no action type configured
```

### Multiple action types (warning)

An action entry with more than one type is technically valid but only the
first will execute. This is flagged as a warning to catch likely
misconfigurations.

```yaml
actions:
  - apply:                  # WARN: action[0] has multiple action types
      file: forjar.yaml
    script: "echo also this"
```

### Empty apply file (error)

An `apply` action must reference a non-empty file path.

```yaml
actions:
  - apply:
      file: ""              # ERROR: action[0] apply.file is empty
```

### Empty script (warning)

A `script` action with a blank command string is flagged as a warning.

```yaml
actions:
  - script: "  "            # WARN: action[0] script is empty
```

### Empty notify channel (error)

A `notify` action must have a non-empty `channel` URL.

```yaml
actions:
  - notify:
      channel: ""           # ERROR: action[0] notify.channel is empty
      message: "something happened"
```

### Cooldown bounds (warning)

A `cooldown_secs` of `0` disables deduplication entirely, which can lead to
rapid-fire triggering when events arrive in bursts. This is flagged as a
warning.

```yaml
rulebooks:
  - name: rapid
    events: [{type: file_changed}]
    actions: [{script: "echo"}]
    cooldown_secs: 0        # WARN: cooldown_secs=0 may cause rapid-fire triggering
```

### High max_retries (warning)

A `max_retries` value above 10 is flagged as unusually high. Excessive
retries can mask underlying failures and delay recovery.

```yaml
rulebooks:
  - name: retry-storm
    events: [{type: manual}]
    actions: [{script: "flaky-command"}]
    max_retries: 50         # WARN: max_retries=50 is unusually high
```

## `forjar rules coverage`

The `coverage` subcommand reports which event types are covered by at least
one rulebook in the file. This helps identify gaps in automation coverage.

```bash
forjar rules coverage -f rulebooks.yaml
```

Output:

```
Event Type Coverage
----------------------------------------
  [+] file_changed: 2 rulebook(s)
  [-] process_exit: 0 rulebook(s)
  [+] cron_fired: 1 rulebook(s)
  [-] webhook_received: 0 rulebook(s)
  [-] metric_threshold: 0 rulebook(s)
  [+] manual: 1 rulebook(s)
```

A `[+]` indicates at least one rulebook handles that event type. A `[-]`
indicates no coverage. The six event types checked are:

| Event Type | Description |
|------------|-------------|
| `file_changed` | File system change (inotify/fanotify) |
| `process_exit` | Process exited (waitpid) |
| `cron_fired` | Cron schedule fired |
| `webhook_received` | HTTP webhook received |
| `metric_threshold` | Metric threshold crossed |
| `manual` | Manual trigger via `forjar trigger` |

## Example Rulebook with Issues

The following rulebook YAML demonstrates several issues that `validate`
will catch:

```yaml
rulebooks:
  # Valid rulebook
  - name: config-repair
    events:
      - type: file_changed
        match:
          path: /etc/nginx/nginx.conf
    actions:
      - apply:
          file: forjar.yaml
          tags: [config]
    cooldown_secs: 60

  # ERROR: duplicate name
  - name: config-repair
    events:
      - type: manual
    actions:
      - script: "forjar apply -f backup.yaml"

  # ERROR: no events
  - name: orphan-actions
    events: []
    actions:
      - script: "echo this never runs"

  # ERROR: empty notify channel + WARN: zero cooldown
  - name: broken-notify
    events:
      - type: process_exit
    actions:
      - notify:
          channel: ""
          message: "process died"
    cooldown_secs: 0

  # WARN: high retries
  - name: retry-heavy
    events:
      - type: metric_threshold
    actions:
      - script: "remediate.sh"
    max_retries: 100
```

Running validation:

```bash
forjar rules validate -f rulebooks.yaml
```

```
Validating rulebooks in rulebooks.yaml
------------------------------------------------------------
5 rulebook(s), 4 error(s), 2 warning(s)
  [ERROR] config-repair: duplicate rulebook name: config-repair
  [ERROR] orphan-actions: no event patterns defined
  [ERROR] broken-notify: action[0] notify.channel is empty
  [WARN ] broken-notify: cooldown_secs=0 may cause rapid-fire triggering
  [WARN ] retry-heavy: max_retries=100 is unusually high
  [ERROR] orphan-actions: no actions defined

Validation FAILED.
```

## JSON Output Mode for CI

Both `validate` and `coverage` support `--json` for machine-readable output,
suitable for CI pipeline integration.

### Validate JSON

```bash
forjar rules validate -f rulebooks.yaml --json
```

```json
{
  "rulebook_count": 3,
  "errors": 1,
  "warnings": 1,
  "passed": false,
  "issues": [
    {
      "rulebook": "bad-notify",
      "severity": "error",
      "message": "action[0] notify.channel is empty"
    },
    {
      "rulebook": "rapid-fire",
      "severity": "warning",
      "message": "cooldown_secs=0 may cause rapid-fire triggering"
    }
  ]
}
```

### Coverage JSON

```bash
forjar rules coverage -f rulebooks.yaml --json
```

```json
{
  "file_changed": 2,
  "process_exit": 0,
  "cron_fired": 1,
  "webhook_received": 0,
  "metric_threshold": 0,
  "manual": 1
}
```

### CI integration example

Use `--json` with `jq` to gate deployments on rulebook validity:

```bash
# Fail CI if any validation errors
forjar rules validate -f rulebooks.yaml --json | jq -e '.passed' || exit 1

# Warn if event types are uncovered
UNCOVERED=$(forjar rules coverage -f rulebooks.yaml --json \
  | jq '[to_entries[] | select(.value == 0)] | length')
if [ "$UNCOVERED" -gt 0 ]; then
  echo "WARNING: $UNCOVERED event type(s) have no rulebook coverage"
fi
```

## Summary of Checks

| Check | Severity | Condition |
|-------|----------|-----------|
| Duplicate name | Error | Two rulebooks share the same `name` |
| Empty events | Error | `events` list is empty |
| Empty actions | Error | `actions` list is empty |
| No action type | Error | Action entry has no `apply`/`destroy`/`script`/`notify` |
| Empty apply file | Error | `apply.file` is an empty string |
| Empty notify channel | Error | `notify.channel` is an empty string |
| Multiple action types | Warning | Action entry has more than one type |
| Empty script | Warning | `script` is blank or whitespace-only |
| Zero cooldown | Warning | `cooldown_secs` is `0` |
| High retries | Warning | `max_retries` exceeds `10` |
