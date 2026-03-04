# Operational Intelligence

Forjar's operational intelligence features provide proactive analysis of your infrastructure configuration, helping you understand complexity, predict failures, and measure blast radius before changes hit production.

## Configuration Complexity Analysis

Score your configuration's complexity across seven dimensions:

```bash
forjar complexity -f forjar.yaml
```

Output:
```
Configuration Complexity Analysis

  Resources:      15
  Machines:       3
  DAG depth:      4
  Cross-machine:  3
  Templates:      5
  Conditionals:   2
  Includes:       2

  Score: 49/100  Grade: C

  Recommendations:
    - Consider grouping related resources by machine
```

### Scoring Dimensions

| Dimension | Weight | Cap | Description |
|-----------|--------|-----|-------------|
| Resources | 1x | 30 | Total resource count |
| DAG depth | 5x | 20 | Longest dependency chain |
| Cross-machine | 3x | 15 | Dependencies across machines |
| Templates | 2x | 10 | Template interpolation count |
| Conditionals | 2x | 10 | `when:` conditional resources |
| Includes | 3x | 10 | Include file depth |
| Machines | 2x | 5 | Machine count |

Grades: **A** (0-20), **B** (21-40), **C** (41-60), **D** (61-80), **F** (81-100).

### JSON Output

```bash
forjar complexity -f forjar.yaml --json
```

## Dependency Impact Analysis

Compute the blast radius of changing a specific resource:

```bash
forjar impact -f forjar.yaml -r db-pkg
```

Output:
```
Dependency Impact Analysis

  Source:    db-pkg
  Risk:      low
  Affected:  2 resource(s)
  Machines:  1 machine(s)
  Est. cascade: 12s

  > web-conf [file] on web (~2s)
    > web-svc [service] on web (~10s)
```

The command performs BFS through the reverse dependency graph, computing:
- **Affected resources** with depth tracking
- **Machine spread** — how many machines are impacted
- **Estimated cascade time** — sum of estimated apply times
- **Risk level** — none/low/medium/high/critical based on count

### Risk Levels

| Level | Affected Resources |
|-------|--------------------|
| none | 0 |
| low | 1-3 |
| medium | 4-10 |
| high | 11-25 |
| critical | 25+ |

## Drift Prediction

Analyze historical event logs to predict which resources are most likely to drift:

```bash
forjar drift-predict --state-dir state/
```

Output:
```
Drift Prediction Report

  Analyzed:   3 resource(s)
  High risk:  1

  nginx-conf on web — risk 78.0% | drifts 4/5 | trend increasing | mtbd 1166s
  cron-job on worker — risk 32.5% | drifts 1/2 | trend stable | mtbd 0s
  postgres-conf on db — risk 0.0% | drifts 0/2 | trend stable | mtbd 0s
```

### Options

```bash
forjar drift-predict --machine web     # Filter to specific machine
forjar drift-predict --limit 5         # Show top 5 predictions
forjar drift-predict --json            # JSON output for CI integration
```

### Risk Score Algorithm

```
risk = min(1.0, (drift_rate * 0.5 + min(0.3, drift_count * 0.05)) * trend_multiplier)
```

Where:
- `drift_rate` = drift events / total events
- `trend_multiplier` = 1.3 (increasing), 0.7 (decreasing), 1.0 (stable)
- Trend is computed by comparing drift frequency in the first vs second half of the timeline
