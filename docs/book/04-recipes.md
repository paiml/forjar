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

## Advanced Recipe Patterns

### Multi-Service Recipe

A recipe can compose multiple services with internal dependencies:

```yaml
# recipes/app-stack.yaml
recipe:
  name: app-stack
  version: "2.0"
  description: "Full application stack: nginx + app + redis"
  inputs:
    app_name:
      type: string
      description: "Application name (used for paths and service names)"
    app_port:
      type: int
      default: 8080
    redis_port:
      type: int
      default: 6379

resources:
  packages:
    type: package
    provider: apt
    packages: [nginx, redis-server]

  app-dir:
    type: file
    state: directory
    path: "/opt/{{inputs.app_name}}"
    mode: "0755"
    depends_on: [packages]

  nginx-config:
    type: file
    path: "/etc/nginx/sites-enabled/{{inputs.app_name}}"
    content: |
      upstream app {
        server 127.0.0.1:{{inputs.app_port}};
      }
      server {
        listen 80;
        location / { proxy_pass http://app; }
      }
    depends_on: [packages]

  redis-config:
    type: file
    path: /etc/redis/redis.conf
    content: |
      bind 127.0.0.1
      port {{inputs.redis_port}}
      maxmemory 256mb
    depends_on: [packages]

  nginx-svc:
    type: service
    name: nginx
    state: running
    restart_on: [nginx-config]
    depends_on: [nginx-config]

  redis-svc:
    type: service
    name: redis-server
    state: running
    restart_on: [redis-config]
    depends_on: [redis-config]
```

Use it:

```yaml
resources:
  my-app:
    type: recipe
    machine: web-server
    recipe: app-stack
    inputs:
      app_name: myapp
      app_port: 3000
```

This expands to 6 resources with the correct dependency chain: `my-app/packages` → `my-app/app-dir` → ... → `my-app/nginx-svc`.

### SSL/TLS Certificate Recipe

A recipe that manages TLS certificates with renewal logic:

```yaml
# recipes/tls-cert.yaml
recipe:
  name: tls-cert
  version: "1.0"
  description: "TLS certificate with auto-renewal via certbot"
  inputs:
    domain:
      type: string
      description: "Domain name for the certificate"
    email:
      type: string
      description: "Email for Let's Encrypt notifications"
    webroot:
      type: path
      default: /var/www/html
      description: "Webroot for ACME challenge"

resources:
  certbot-pkg:
    type: package
    provider: apt
    packages: [certbot]

  cert-dir:
    type: file
    state: directory
    path: "/etc/letsencrypt/live/{{inputs.domain}}"
    mode: "0700"
    depends_on: [certbot-pkg]

  renewal-cron:
    type: cron
    name: "certbot-renew-{{inputs.domain}}"
    schedule: "0 3 * * 1"
    command: "certbot renew --webroot -w {{inputs.webroot}} --quiet"
    owner: root
    depends_on: [certbot-pkg]
```

### GPU Inference Server Recipe (FJ-243)

A production recipe for deploying ML model inference on GPU machines:

```yaml
# recipes/apr-inference-server.yaml
recipe:
  name: apr-inference-server
  version: "1.0"
  inputs:
    model_source:
      type: string
      description: "HuggingFace repo ID or URL"
    model_format:
      type: enum
      choices: [gguf, safetensors, apr]
      default: gguf
    quantization:
      type: string
      default: q4_k_m
    port:
      type: int
      default: 8080
    workers:
      type: int
      default: 1
    gpu_device:
      type: int
      default: 0
    user:
      type: string
      default: apr

resources:
  gpu-driver:
    type: gpu
    driver_version: "550"
    cuda_version: "12.4"
    persistence_mode: true

  model-download:
    type: model
    name: "{{inputs.model_source}}"
    source: "{{inputs.model_source}}"
    format: "{{inputs.model_format}}"
    path: "/opt/apr/models/{{inputs.model_format}}"
    depends_on: [gpu-driver]

  apr-systemd-unit:
    type: file
    path: /etc/systemd/system/apr-serve.service
    content: |
      [Unit]
      Description=Aprender Inference Server
      After=network.target
      [Service]
      ExecStart=/usr/local/bin/apr serve --model /opt/apr/models/{{inputs.model_format}} --port {{inputs.port}}
      [Install]
      WantedBy=multi-user.target
    depends_on: [model-download]

  apr-serve:
    type: service
    name: apr-serve
    state: running
    enabled: true
    restart_on: [apr-systemd-unit, model-download]

  apr-firewall:
    type: network
    port: "{{inputs.port}}"
    protocol: tcp
    action: allow
```

Usage in `forjar.yaml`:

```yaml
resources:
  inference:
    type: recipe
    machine: gpu-box
    recipe: apr-inference-server
    inputs:
      model_source: "TheBloke/Llama-2-7B-GGUF"
      model_format: gguf
      port: 8080
```

This expands to 8 resources (gpu driver, model download, service account, data directory, systemd unit, service, firewall, health check) with correct dependency ordering.

### Recipe with External Dependencies

Recipes can declare dependencies on resources outside the recipe:

```yaml
# forjar.yaml
resources:
  base-packages:
    type: package
    machine: web
    provider: apt
    packages: [curl, jq]

  web:
    type: recipe
    machine: web
    recipe: web-server
    depends_on: [base-packages]
    inputs:
      domain: example.com
```

The `depends_on: [base-packages]` on the recipe resource makes ALL expanded resources depend on `base-packages`. This is the mechanism for cross-recipe dependencies.

### Reusing Recipes Across Machines

The same recipe can be instantiated multiple times with different parameters:

```yaml
resources:
  staging-web:
    type: recipe
    machine: staging
    recipe: web-server
    inputs:
      domain: staging.example.com
      port: 8080
      log_level: debug

  production-web:
    type: recipe
    machine: production
    recipe: web-server
    inputs:
      domain: www.example.com
      port: 80
      log_level: error
```

Each instantiation creates a namespaced set of resources: `staging-web/nginx-pkg`, `staging-web/site-config`, etc. This prevents ID collisions.

### Recipe Expansion Order

The expansion pipeline processes recipes in this order:

```
1. Load forjar.yaml
2. For each recipe resource:
   a. Load recipes/{name}.yaml
   b. Validate inputs against declared schema
   c. Resolve {{inputs.X}} templates in resource fields
   d. Namespace resource IDs: "{parent-id}/{resource-id}"
   e. Set machine field from parent recipe resource
   f. Add external depends_on to first resource in chain
3. Replace recipe resources with expanded resources
4. Validate expanded config (deps, machines, types)
```

## Recipe Testing

### Unit Testing a Recipe

Test a recipe in isolation using a container machine:

```yaml
# test-web-server.yaml
version: "1.0"
name: test-web-server-recipe
machines:
  test:
    hostname: test
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      ephemeral: true

resources:
  web:
    type: recipe
    machine: test
    recipe: web-server
    inputs:
      domain: test.local
      port: 8080
      log_level: debug
```

```bash
# Validate recipe expansion
forjar validate -f test-web-server.yaml

# Apply to container
forjar apply -f test-web-server.yaml --state-dir /tmp/recipe-test

# Check for drift (should be clean immediately after apply)
forjar drift -f test-web-server.yaml --state-dir /tmp/recipe-test
```

### Recipe CI Pipeline

Add recipe validation to CI:

```yaml
# .github/workflows/recipe-test.yml
name: Recipe Tests
on: [pull_request]
jobs:
  validate-recipes:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build forjar
        run: cargo build --release
      - name: Validate all configs that use recipes
        run: |
          for config in examples/*.yaml; do
            echo "Validating $config..."
            ./target/release/forjar validate -f "$config"
          done
      - name: Test recipe expansion
        run: |
          ./target/release/forjar plan -f examples/recipe-example.yaml --state-dir /tmp/state
```

### Recipe Development Workflow

The recommended workflow for creating and iterating on recipes:

```
1. Write recipe YAML in recipes/
2. Create a test config with container machine
3. forjar validate -f test-config.yaml
4. forjar plan -f test-config.yaml --state-dir /tmp/test
5. forjar apply -f test-config.yaml --state-dir /tmp/test
6. forjar drift -f test-config.yaml --state-dir /tmp/test
7. Iterate on recipe, re-apply until clean
8. Move test config inputs to production config
```

## Recipe Internals

### Input Type Coercion

Recipe inputs are type-checked at parse time. The coercion rules:

| Declared Type | YAML Value | Result |
|---------------|-----------|--------|
| `string` | `"hello"` | `"hello"` |
| `string` | `42` | `"42"` (auto-coerced) |
| `int` | `42` | `42` |
| `int` | `"42"` | Error: expected int |
| `bool` | `true` | `true` |
| `bool` | `"yes"` | Error: expected bool |
| `path` | `"/etc/app"` | `"/etc/app"` |
| `path` | `"relative/path"` | Error: must be absolute |
| `enum` | `"warn"` | `"warn"` (if in choices) |
| `enum` | `"verbose"` | Error: not in choices |

### Template Resolution Order

Templates inside recipes are resolved in two passes:

**Pass 1: Recipe-level** (`{{inputs.X}}`)
- Resolved during recipe expansion
- Uses values from the calling config's `inputs:` block
- Results in concrete values before the resource enters the main config

**Pass 2: Config-level** (`{{params.X}}`, `{{secrets.X}}`, `{{machine.X.Y}}`)
- Resolved during the main template resolution phase
- Uses the calling config's `params:`, `secrets`, and `machines:` blocks
- Happens after recipe expansion

This means a recipe can mix both template types:

```yaml
# In a recipe resource
content: |
  app_name={{inputs.app_name}}
  environment={{params.env}}
  db_host={{machine.db.addr}}
  api_key={{secrets.api-key}}
```

`{{inputs.app_name}}` is resolved in Pass 1. The rest are resolved in Pass 2.

### Namespace Collision Prevention

Recipe namespacing prevents ID collisions when the same recipe is used multiple times:

```yaml
resources:
  web1-stack:
    type: recipe
    machine: web1
    recipe: web-server
    inputs: { domain: a.com }
  web2-stack:
    type: recipe
    machine: web2
    recipe: web-server
    inputs: { domain: b.com }
```

Expanded IDs:
- `web1-stack/nginx-pkg` (machine: web1)
- `web1-stack/site-config` (machine: web1)
- `web2-stack/nginx-pkg` (machine: web2)
- `web2-stack/site-config` (machine: web2)

Internal `depends_on` and `restart_on` references are also namespaced. If the recipe has `depends_on: [nginx-pkg]`, it becomes `depends_on: [web1-stack/nginx-pkg]` after expansion.

### Recipe File Resolution

Forjar searches for recipe files relative to the config file's directory:

```
forjar.yaml          ← config file
recipes/
  web-server.yaml    ← found via "recipe: web-server"
  database.yaml      ← found via "recipe: database"
  monitoring.yaml    ← found via "recipe: monitoring"
```

If `forjar.yaml` is at `/opt/infra/forjar.yaml`, then `recipe: web-server` loads `/opt/infra/recipes/web-server.yaml`.

## Recipe Anti-Patterns

### Avoid: Recipes Without Inputs

```yaml
# BAD — recipe with no parameterization (just use regular resources)
recipe:
  name: static-config
  version: "1.0"
  # No inputs — this recipe does the same thing every time

# GOOD — add inputs for the parts that vary
recipe:
  name: configurable-stack
  inputs:
    app_name: { type: string }
    port: { type: int, default: 8080 }
```

If a recipe has zero inputs, it's just adding indirection. Use regular resources instead.

### Avoid: Deeply Nested Dependencies

```yaml
# BAD — 10-resource chain with serial dependencies
resources:
  step-1: { depends_on: [] }
  step-2: { depends_on: [step-1] }
  step-3: { depends_on: [step-2] }
  # ... 7 more steps ...

# GOOD — use parallel-safe dependency structure
resources:
  packages: {}
  config-a: { depends_on: [packages] }
  config-b: { depends_on: [packages] }
  service-a: { depends_on: [config-a] }
  service-b: { depends_on: [config-b] }
```

Prefer wide DAGs (many resources depending on one) over deep chains (serial sequences). Wide DAGs enable future parallel execution.

### Avoid: Environment-Specific Recipes

```yaml
# BAD — separate recipes for each environment
recipe: web-server-production
recipe: web-server-staging

# GOOD — one recipe, parameterize differences
recipe: web-server
inputs:
  log_level: { type: enum, choices: [error, warn, info, debug] }
  port: { type: int }
```

Use inputs to handle environment differences, not separate recipe files.

## Recipe Testing with bashrs

Every resource forjar manages produces shell scripts (check, apply, state_query). When
those resources originate from a recipe, the generated scripts carry the same shell
safety requirements as any other resource. bashrs validates recipe-generated scripts
through the same purification pipeline described in the architecture chapter.

### Why Recipe Scripts Need Validation

Recipes introduce an extra layer of template expansion. A recipe's `{{inputs.X}}`
values are injected into shell script templates before execution. If an input contains
shell metacharacters, whitespace, or unexpected values, the resulting script could
break or behave unsafely. bashrs catches these problems statically.

### Validating Recipe Scripts with `forjar lint`

The `forjar lint` command generates scripts for all resources -- including expanded
recipe resources -- and passes each through the bashrs linter:

```bash
forjar lint -f forjar.yaml
```

Sample output for a config using the `web-server` recipe:

```
  warn: bashrs: web/nginx-pkg/apply [SC2086] Double quote to prevent globbing
  warn: bashrs: web/site-config/apply [SC2059] Use %s with printf
  warn: bashrs script lint: 0 error(s), 2 warning(s) across 3 resources
No lint errors found.
```

The resource IDs in the output use the namespaced form (`web/nginx-pkg`,
`web/site-config`) so you can trace each diagnostic back to the recipe resource that
produced it.

### Auditing Expanded Scripts

Write the generated scripts to disk for manual review:

```bash
forjar plan -f forjar.yaml --output-dir /tmp/audit/
ls /tmp/audit/web-server/
```

Output:

```
web/nginx-pkg.check.sh
web/nginx-pkg.apply.sh
web/nginx-pkg.state_query.sh
web/site-config.check.sh
web/site-config.apply.sh
web/site-config.state_query.sh
web/nginx-svc.check.sh
web/nginx-svc.apply.sh
web/nginx-svc.state_query.sh
```

Each script is a standalone shell file. Open any of them to see exactly what would
execute on the target machine after recipe expansion and template resolution.

### The Purification Pipeline for Recipe Scripts

When `forjar apply` runs, recipe-generated scripts pass through this pipeline:

```
1. Recipe expansion     — {{inputs.X}} resolved from caller's inputs block
2. Template resolution  — {{params.X}}, {{secrets.X}} resolved from config
3. Script generation    — codegen produces check/apply/state_query shell
4. bashrs validation    — validate_script() checks for Error-severity issues
5. Transport execution  — script piped to bash on target machine
```

If step 4 fails, the resource is marked as failed, execution halts for that machine
(Jidoka), and the error is recorded in the event log.

### Testing Recipe Inputs for Shell Safety

When writing a recipe, test it with edge-case inputs to verify that the generated
scripts remain valid shell:

```yaml
# test-edge-cases.yaml
version: "1.0"
name: recipe-edge-test
machines:
  test:
    hostname: test
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      ephemeral: true

resources:
  # Test with spaces in domain (should produce valid shell)
  edge-web:
    type: recipe
    machine: test
    recipe: web-server
    inputs:
      domain: "my site.example.com"
      port: 8080
      log_level: debug
```

Run lint to check whether the expanded scripts handle the space correctly:

```bash
forjar lint -f test-edge-cases.yaml
```

If bashrs reports errors, the recipe's template needs quoting fixes in its resource
definitions. For example, ensure file paths in the recipe use single-quoted heredocs
rather than unquoted variable expansion.

### CI Integration for Recipe Script Validation

Add bashrs validation to your recipe CI pipeline:

```yaml
# .github/workflows/recipe-lint.yml
name: Recipe Script Lint
on: [pull_request]
jobs:
  lint-recipes:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build forjar
        run: cargo build --release
      - name: Lint all configs (includes bashrs script checks)
        run: |
          for config in examples/*.yaml; do
            echo "Linting $config..."
            ./target/release/forjar lint -f "$config" --json | \
              jq -e '.warnings_count == 0' || echo "WARN: $config has lint findings"
          done
      - name: Fail on script errors
        run: |
          for config in examples/*.yaml; do
            # --json output includes bashrs diagnostics
            errors=$(./target/release/forjar lint -f "$config" --json | \
              jq '.warnings | map(select(startswith("bashrs") and contains("Error"))) | length')
            if [ "$errors" -gt 0 ]; then
              echo "ERROR: $config has $errors bashrs errors"
              exit 1
            fi
          done
```

This catches recipe script problems before they reach production machines.

## Recipe Composition Patterns

Recipes become powerful when composed together to build complete infrastructure stacks.
This section covers patterns for combining multiple recipes into cohesive deployments.

### Pattern 1: Web Stack (Package + Config + Service)

The most common composition is a web application stack built from three focused
recipes:

```yaml
# recipes/base-packages.yaml
recipe:
  name: base-packages
  version: "1.0"
  description: "Common packages for all machines"
  inputs:
    extras:
      type: string
      default: ""
      description: "Space-separated list of additional packages"

resources:
  core:
    type: package
    provider: apt
    packages: [curl, jq, htop, unzip]
```

```yaml
# recipes/nginx-site.yaml
recipe:
  name: nginx-site
  version: "1.0"
  description: "Nginx virtual host with optional SSL"
  inputs:
    domain:
      type: string
    port:
      type: int
      default: 80
    upstream_port:
      type: int
      default: 8080

resources:
  nginx-pkg:
    type: package
    provider: apt
    packages: [nginx]

  site-config:
    type: file
    path: "/etc/nginx/sites-enabled/{{inputs.domain}}"
    content: |
      upstream app {
        server 127.0.0.1:{{inputs.upstream_port}};
      }
      server {
        listen {{inputs.port}};
        server_name {{inputs.domain}};
        location / { proxy_pass http://app; }
      }
    owner: root
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

```yaml
# recipes/app-service.yaml
recipe:
  name: app-service
  version: "1.0"
  description: "Systemd-managed application service"
  inputs:
    app_name:
      type: string
    app_port:
      type: int
      default: 8080
    app_binary:
      type: path
      default: /usr/local/bin/app

resources:
  app-dir:
    type: file
    state: directory
    path: "/opt/{{inputs.app_name}}"
    mode: "0755"

  unit-file:
    type: file
    path: "/etc/systemd/system/{{inputs.app_name}}.service"
    content: |
      [Unit]
      Description={{inputs.app_name}}
      After=network.target

      [Service]
      ExecStart={{inputs.app_binary}} --port {{inputs.app_port}}
      WorkingDirectory=/opt/{{inputs.app_name}}
      Restart=on-failure

      [Install]
      WantedBy=multi-user.target
    owner: root
    mode: "0644"
    depends_on: [app-dir]

  app-svc:
    type: service
    name: "{{inputs.app_name}}"
    state: running
    enabled: true
    restart_on: [unit-file]
    depends_on: [unit-file]
```

Compose them in `forjar.yaml`:

```yaml
version: "1.0"
name: web-stack
machines:
  web1:
    hostname: web1
    addr: 10.0.0.1
    user: deploy
    ssh_key: ~/.ssh/id_ed25519

resources:
  base:
    type: recipe
    machine: web1
    recipe: base-packages
    inputs: {}

  app:
    type: recipe
    machine: web1
    recipe: app-service
    depends_on: [base]
    inputs:
      app_name: myapi
      app_port: 3000
      app_binary: /usr/local/bin/myapi

  frontend:
    type: recipe
    machine: web1
    recipe: nginx-site
    depends_on: [app]
    inputs:
      domain: api.example.com
      port: 80
      upstream_port: 3000
```

The resulting DAG:

```
base/core
  |
  v
app/app-dir -> app/unit-file -> app/app-svc
                                     |
                                     v
                          frontend/nginx-pkg -> frontend/site-config -> frontend/nginx-svc
```

The `depends_on` on each recipe resource creates a sequential chain: base packages
install first, then the application deploys, then nginx configures itself as a reverse
proxy.

### Pattern 2: Layered Infrastructure (Base + Role + App)

Separate concerns into layers that apply in order:

```yaml
version: "1.0"
name: production
machines:
  web1: { hostname: web1, addr: 10.0.0.1, user: deploy }
  web2: { hostname: web2, addr: 10.0.0.2, user: deploy }
  db1:  { hostname: db1,  addr: 10.0.0.3, user: deploy }

resources:
  # Layer 1: Base (all machines)
  web1-base:
    type: recipe
    machine: web1
    recipe: base-packages
    inputs: {}
  web2-base:
    type: recipe
    machine: web2
    recipe: base-packages
    inputs: {}
  db1-base:
    type: recipe
    machine: db1
    recipe: base-packages
    inputs: {}

  # Layer 2: Role-specific recipes
  web1-nginx:
    type: recipe
    machine: web1
    recipe: nginx-site
    depends_on: [web1-base]
    inputs:
      domain: www.example.com
      upstream_port: 3000
  web2-nginx:
    type: recipe
    machine: web2
    recipe: nginx-site
    depends_on: [web2-base]
    inputs:
      domain: www.example.com
      upstream_port: 3000

  # Layer 3: Application-specific resources
  web1-app:
    type: recipe
    machine: web1
    recipe: app-service
    depends_on: [web1-nginx]
    inputs:
      app_name: frontend
      app_port: 3000
  web2-app:
    type: recipe
    machine: web2
    recipe: app-service
    depends_on: [web2-nginx]
    inputs:
      app_name: frontend
      app_port: 3000
```

Each layer depends on the one below it. The base recipe runs on every machine. The
role recipe (nginx) runs only on web servers. The app recipe runs last.

### Pattern 3: Sidecar Composition (App + Monitoring + Logging)

Attach cross-cutting concerns as sibling recipes that share a common base dependency:

```yaml
resources:
  base:
    type: recipe
    machine: web1
    recipe: base-packages
    inputs: {}

  app:
    type: recipe
    machine: web1
    recipe: app-service
    depends_on: [base]
    inputs:
      app_name: myapi
      app_port: 3000

  monitoring:
    type: recipe
    machine: web1
    recipe: monitoring
    depends_on: [base]
    inputs:
      metrics_port: 9100

  logging:
    type: recipe
    machine: web1
    recipe: logging
    depends_on: [base]
    inputs:
      log_path: /var/log/myapi
      retention_days: 14
```

The dependency graph is wide, not deep:

```
        base/core
       /    |    \
      v     v     v
    app  monitoring  logging
```

The app, monitoring, and logging recipes all depend on `base` but not on each other.
This is the preferred structure: it enables future parallel execution and keeps each
recipe independently testable.

### Pattern 4: Sovereign AI Stack (Multi-Machine Composition)

The most complex composition: GPU inference + distributed workers + observability across multiple machines:

```yaml
# examples/dogfood-sovereign-stack.yaml
machines:
  gpu-box:       { addr: container, roles: [gpu-compute] }
  worker-1:      { addr: container, roles: [worker] }
  monitor:       { addr: container, roles: [observability] }

resources:
  observability:
    type: recipe
    machine: monitor
    recipe: renacer-observability
    inputs: { grafana_port: 3000, jaeger_port: 16686 }

  worker:
    type: recipe
    machine: worker-1
    recipe: repartir-worker
    inputs: { listen_port: 9000, tls_cert: certs/cert.pem, tls_key: certs/key.pem }

  inference:
    type: recipe
    machine: gpu-box
    recipe: apr-inference-server
    inputs: { model_source: "TheBloke/Llama-2-7B-GGUF", port: 8080 }

  coordination:
    type: recipe
    machine: monitor
    recipe: sovereign-ai-stack
    inputs: { model_source: "TheBloke/Llama-2-7B-GGUF", api_port: 8080 }
```

This expands to **33 resources** across 3 machines with `parallel_machines: true`:

```
gpu-box (8 resources):   GPU driver → model → systemd unit → service → firewall
worker-1 (10 resources): TLS certs → config → systemd unit → service → firewall
monitor (15 resources):  Jaeger → Grafana → renacer → coordination → health cron
```

### Composition Guidelines

| Guideline | Rationale |
|-----------|-----------|
| Use `depends_on` on recipe resources to define inter-recipe ordering | External deps are injected into the first expanded resource |
| Keep recipes independent when possible | Wide DAGs are faster and easier to debug than deep chains |
| One recipe per concern (packages, config, service) | Smaller recipes compose more flexibly than monolithic ones |
| Use consistent input naming across recipes | `app_name`, `port`, `domain` reappearing across recipes reduces cognitive load |
| Test composed stacks with container transport | Validates the full expansion and dependency chain before production |

### Verifying Composition

After composing recipes, verify the expanded dependency graph:

```bash
# Visualize the full DAG (Mermaid format)
forjar graph -f forjar.yaml

# Validate all recipe inputs and expansions
forjar validate -f forjar.yaml

# Preview the plan to see resource ordering
forjar plan -f forjar.yaml --state-dir state/

# Lint all generated scripts (includes bashrs checks)
forjar lint -f forjar.yaml
```

The graph command is particularly useful for composed stacks. It shows every expanded
resource and its dependency edges, making it straightforward to verify that the
execution order matches your intent.
