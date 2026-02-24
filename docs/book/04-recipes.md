# Recipes

Recipes are reusable, parameterized resource patterns. Think Homebrew formulae for infrastructure.

## Recipe File Format

```yaml
# recipes/web-server.yaml
name: web-server
version: "1.0"
description: "Nginx web server with config"

inputs:
  domain:
    type: string
    required: true
    description: "Server domain name"
  port:
    type: int
    default: 80
    min: 1
    max: 65535
  ssl:
    type: bool
    default: false
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
      }
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
| `enum` | `choices` (required) | `log_level: warn` |
| `list` | `min_length`, `max_length` | `tags: [a, b]` |

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
  web:
    type: recipe
    machine: web1
    recipe: web-server
    inputs:
      domain: example.com
      port: 443
      ssl: true
      log_level: info
```

## How Expansion Works

When forjar loads the config:

1. Recipe YAML is parsed and validated
2. Inputs are type-checked against declared types (`string`, `int`, `bool`, `path`, `enum`)
3. Missing inputs get default values
4. Required inputs without defaults produce errors
5. Resources are expanded with namespace prefix: `web/nginx-pkg`, `web/site-config`, `web/nginx-svc`
6. `{{inputs.X}}` templates are resolved with validated values
7. Expanded resources are merged into the main resource set
8. External `depends_on` on the recipe ID resolves to the recipe's last resource

## Namespacing

Recipe resources are namespaced by the resource ID that references them. If you declare:

```yaml
resources:
  web:
    type: recipe
    recipe: web-server
    inputs: { domain: example.com }
```

The expanded resources become: `web/nginx-pkg`, `web/site-config`, `web/nginx-svc`.

Internal `depends_on` references are also namespaced automatically.

## Composition

Recipes can require other recipes:

```yaml
# recipes/app-stack.yaml
name: app-stack
version: "1.0"
requires:
  - recipes/web-server.yaml
  - recipes/database.yaml

inputs:
  app_name:
    type: string
    required: true

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
