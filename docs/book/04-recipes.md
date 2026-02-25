# Recipes

Recipes are reusable, parameterized resource patterns. Think Homebrew formulae for infrastructure.

## Recipe File Format

Recipe files live in `recipes/` next to your `forjar.yaml`:

```yaml
# recipes/web-server.yaml
recipe:
  name: web-server
  version: "1.0"
  description: "Nginx web server with config"
  inputs:
    domain:
      type: string
      description: "Server domain name"
    port:
      type: int
      default: 80
      min: 1
      max: 65535
    log_level:
      type: enum
      choices: [error, warn, info, debug]
      default: warn

resources:
  nginx-pkg:
    type: package
    provider: apt
    packages: [nginx]

  site-config:
    type: file
    path: "/etc/nginx/sites-enabled/{{inputs.domain}}"
    content: |
      server {
        listen {{inputs.port}};
        server_name {{inputs.domain}};
        error_log /var/log/nginx/error.log {{inputs.log_level}};
        root /var/www/{{inputs.domain}};
      }
    owner: root
    group: root
    mode: "0644"
    depends_on: [nginx-pkg]

  nginx-svc:
    type: service
    name: nginx
    state: running
    enabled: true
    restart_on: [site-config]
    depends_on: [site-config]
```

## Input Types

| Type | Constraints | Example |
|------|------------|---------|
| `string` | — | `domain: "example.com"` |
| `int` | `min`, `max` | `port: 8080` |
| `bool` | — | `ssl: true` |
| `path` | Must start with `/` | `cert: /etc/ssl/cert.pem` |
| `enum` | `choices` | `log_level: warn` |

All inputs support:
- `default: value` (used when input not provided)
- `description: "..."` (documentation)

## Using Recipes

Reference a recipe in your `forjar.yaml`:

```yaml
version: "1.0"
name: production

machines:
  web1:
    hostname: web1
    addr: 10.0.0.1

resources:
  base-tools:
    type: package
    machine: web1
    provider: apt
    packages: [curl, jq]

  web:
    type: recipe
    machine: web1
    recipe: web-server
    depends_on:
      - base-tools
    inputs:
      domain: example.com
      port: 443
      log_level: info
```

## How Expansion Works

When forjar loads the config (via `validate`, `plan`, `apply`, `drift`, etc.):

1. Config YAML is parsed and validated
2. Recipe resources (`type: recipe`) are detected
3. Recipe file is loaded from `recipes/{name}.yaml` relative to the config
4. Inputs are type-checked against declared types
5. Missing inputs get default values; required inputs without defaults produce errors
6. Resources are expanded with namespace prefix: `web/nginx-pkg`, `web/site-config`, `web/nginx-svc`
7. `{{inputs.X}}` templates are resolved with validated values
8. External `depends_on` from the recipe resource are injected into the first expanded resource
9. Internal `depends_on` references are namespaced automatically
10. Expanded resources replace the recipe resource in the config

The expansion happens in the `parse_and_validate` pipeline, so all downstream commands (plan, apply, drift, graph) see the fully expanded resources.

## Viewing Expanded Resources

```bash
# Show expanded plan
forjar plan -f forjar.yaml --state-dir state/

# Show dependency graph (Mermaid format)
forjar graph -f forjar.yaml

# Validate config (shows expanded resource count)
forjar validate -f forjar.yaml
```

## Namespacing

Recipe resources are namespaced by the resource ID that references them:

```yaml
resources:
  web:
    type: recipe
    recipe: web-server
    inputs: { domain: example.com }
```

Expanded resources: `web/nginx-pkg`, `web/site-config`, `web/nginx-svc`.

Internal `depends_on` and `restart_on` references are also namespaced automatically.

## Composition

Recipes can require other recipes:

```yaml
# recipes/app-stack.yaml
recipe:
  name: app-stack
  version: "1.0"
  requires:
    - recipe: web-server
    - recipe: database

  inputs:
    app_name:
      type: string

resources:
  app-config:
    type: file
    path: "/etc/{{inputs.app_name}}/config.yaml"
    content: "name: {{inputs.app_name}}"
```

## Sharing Recipes

Recipes are plain YAML files. Share them via:

- **Git**: Check recipes into your repo under `recipes/`
- **Git submodules**: Reference a shared recipe repository
- **Copy**: Just copy the YAML file

No package manager needed. No registry. Just files.

## Step-by-Step: Writing a Recipe

Walk through creating a "monitoring agent" recipe from scratch.

### 1. Create the recipe file

```yaml
# recipes/monitoring.yaml
recipe:
  name: monitoring
  version: "1.0"
  description: "Node exporter + Prometheus scrape endpoint"
  inputs:
    metrics_port:
      type: int
      default: 9100
      min: 1024
      max: 65535
    retention_days:
      type: int
      default: 7
```

### 2. Define resources inside the recipe

```yaml
resources:
  node-exporter-pkg:
    type: package
    provider: apt
    packages: [prometheus-node-exporter]

  config:
    type: file
    path: /etc/default/prometheus-node-exporter
    content: |
      ARGS="--web.listen-address=:{{inputs.metrics_port}}"
    mode: "0644"
    depends_on: [node-exporter-pkg]

  service:
    type: service
    name: prometheus-node-exporter
    state: running
    enabled: true
    restart_on: [config]
    depends_on: [config]

  firewall:
    type: network
    port: "{{inputs.metrics_port}}"
    protocol: tcp
    action: allow
    depends_on: [service]
```

### 3. Use the recipe in your config

```yaml
version: "1.0"
name: fleet
machines:
  web1: { hostname: web1, addr: 10.0.0.1 }
  web2: { hostname: web2, addr: 10.0.0.2 }
  db1:  { hostname: db1,  addr: 10.0.0.3 }

resources:
  web1-mon:
    type: recipe
    machine: web1
    recipe: monitoring
    inputs: { metrics_port: 9100 }

  web2-mon:
    type: recipe
    machine: web2
    recipe: monitoring
    inputs: { metrics_port: 9100 }

  db-mon:
    type: recipe
    machine: db1
    recipe: monitoring
    inputs: { metrics_port: 9200, retention_days: 30 }
```

### 4. Verify expansion

```bash
# Validate the config (recipes are expanded during validation)
forjar validate -f forjar.yaml

# See the full dependency graph
forjar graph -f forjar.yaml

# Preview the plan
forjar plan -f forjar.yaml --state-dir state/
```

The expanded resources will be:
- `web1-mon/node-exporter-pkg`
- `web1-mon/config`
- `web1-mon/service`
- `web1-mon/firewall`
- `web2-mon/node-exporter-pkg` (same pattern)
- `db-mon/node-exporter-pkg` (with port 9200)

### 5. Test locally

```bash
# Use a container transport for safe testing
# Add to machines:
#   test:
#     hostname: test
#     addr: container
#     transport: container
#     container:
#       runtime: docker
#       image: ubuntu:22.04

forjar apply -f forjar.yaml --state-dir state/
forjar drift -f forjar.yaml --state-dir state/
```

## Input Validation

Forjar validates recipe inputs at parse time, before any resources are applied.

### Type Checking Rules

| Input Type | Validation | Error Example |
|------------|-----------|---------------|
| `string` | Any non-null string value | `input 'domain' expected string, got null` |
| `int` | Must parse as integer, must satisfy `min`/`max` | `input 'port' value 70000 exceeds max 65535` |
| `bool` | Must be `true` or `false` | `input 'ssl' expected bool, got 'yes'` |
| `path` | Must start with `/` | `input 'cert' path must be absolute, got 'cert.pem'` |
| `enum` | Must be one of `choices` | `input 'level' must be one of [error, warn, info, debug], got 'verbose'` |

### Missing Input Handling

When a recipe input is not provided by the caller:

1. If the input has a `default:` value, it is used automatically
2. If the input has no default, forjar reports a validation error

```
Error: recipe 'web-server' input 'domain' is required but not provided
```

### Extra Input Detection

Inputs not declared in the recipe are flagged as warnings:

```
Warning: recipe 'web-server' received unknown input 'typo_domain' — ignored
```

## Debugging Recipes

### Expansion Tracing

Use `forjar show` to inspect individual expanded resources:

```bash
# Show a specific expanded resource as JSON
forjar show -f forjar.yaml -r web/nginx-pkg --json

# List all expanded resources from the plan
forjar plan -f forjar.yaml --state-dir state/
```

### Common Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `recipe file not found: recipes/X.yaml` | Recipe YAML doesn't exist | Create `recipes/X.yaml` relative to `forjar.yaml` |
| `input 'Y' is required but not provided` | Missing required input with no default | Add `Y:` to `inputs:` block |
| `input 'Y' value Z exceeds max` | Int input out of range | Use a value within min/max bounds |
| `resource 'web' has type 'recipe' but no recipe name` | Missing `recipe:` field | Add `recipe: <name>` to the resource |
| `circular dependency: web/a → web/b → web/a` | Cycle in recipe's internal deps | Fix `depends_on` inside the recipe file |

### Recipe File Discovery

Forjar searches for recipe files in this order:
1. `recipes/{name}.yaml` relative to the config file's directory
2. The current working directory's `recipes/` folder

## Best Practices

- **One concern per recipe**: A recipe for "monitoring", not "monitoring + logging + alerting"
- **Sensible defaults**: Every input should have a default unless it's truly unique per use
- **Document inputs**: The `description` field shows up in error messages and validation output
- **Test expansion**: Run `forjar validate` after adding a recipe to catch input type mismatches early
- **Namespace awareness**: External `depends_on` uses plain IDs; internal uses are auto-namespaced
- **Version your recipes**: Increment `recipe.version` when inputs or behavior change
- **Test locally first**: Use container transport to verify recipe expansion end-to-end before deploying to production
