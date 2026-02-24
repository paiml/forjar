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
