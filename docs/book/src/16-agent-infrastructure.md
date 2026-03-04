# Agent Infrastructure & pforge Integration

Forjar manages AI agent infrastructure as declarative resources: MCP servers, model deployments, tool permissions, and health monitoring.

## MCP Server as a Resource

Deploy [Model Context Protocol](https://modelcontextprotocol.io) servers as forjar resources:

```yaml
resources:
  - name: filesystem-mcp
    type: mcp_server
    package: "@anthropic/mcp-server-filesystem"
    transport: stdio
    config:
      allowed_paths: ["/data", "/config"]
    ensure: running

  - name: database-mcp
    type: mcp_server
    package: "@anthropic/mcp-server-sqlite"
    transport: sse
    port: 3001
    config:
      database: /data/app.db
    ensure: running
```

## pforge YAML Deployment

Deploy complete pforge agent configurations:

```yaml
resources:
  - name: pforge-config
    type: file
    path: /etc/pforge/config.yaml
    content: |
      name: ops-agent
      model: claude-sonnet-4-6
      mcp_servers:
        - name: filesystem
          command: npx @anthropic/mcp-server-filesystem /data
        - name: database
          command: npx @anthropic/mcp-server-sqlite /data/app.db
      tools:
        allow: [read_file, write_file, query]
        deny: [delete_file, drop_table]

  - name: pforge-service
    type: service
    name: pforge-agent
    command: pforge serve --config /etc/pforge/config.yaml
    depends_on: [pforge-config, filesystem-mcp, database-mcp]
    ensure: running
```

## Agent Deployment Patterns

### Single-Agent with MCP Tools

```yaml
resources:
  - name: gpu-driver
    type: gpu_driver
    gpu_backend: nvidia

  - name: model-download
    type: model
    format: gguf
    source: huggingface://TheBloke/Llama-2-7B-GGUF
    path: /models/llama2

  - name: inference-mcp
    type: mcp_server
    package: custom-inference-server
    config:
      model_path: /models/llama2
    depends_on: [model-download, gpu-driver]

  - name: agent
    type: command
    command: pforge serve --config agent.yaml
    depends_on: [inference-mcp]
```

### Multi-Agent Fleet

```yaml
machines:
  - name: agent-01
    host: 10.0.1.1
  - name: agent-02
    host: 10.0.1.2
  - name: agent-03
    host: 10.0.1.3

resources:
  - name: agent-config
    type: file
    path: /etc/agent/config.yaml
    template: templates/agent-config.yaml
    machine: all

  - name: agent-service
    type: service
    name: pforge-agent
    depends_on: [agent-config]
    machine: all
    ensure: running
```

## Agent Tool Permission Policies

Control which MCP tools agents can access:

```yaml
policy:
  agent_tools:
    allow:
      - read_file
      - list_directory
      - query_database
    deny:
      - delete_file
      - drop_table
      - execute_command
    require_approval:
      - write_file
      - create_table
```

Permissions are enforced at the MCP server level. Denied tools return an error; approval-required tools pause for human confirmation.

## Agent Health Monitoring

Monitor agent fleet health with forjar's drift detection:

```bash
# Check agent health across fleet
forjar status -f agents.yaml --machine-health

# Detect drifted agent configurations
forjar check -f agents.yaml --drift-details

# Watch for real-time changes
forjar status -f agents.yaml --watch
```

Agent health checks verify:
- MCP server processes are running
- Model files exist and have correct BLAKE3 hashes
- Configuration files match desired state
- GPU drivers are at expected versions

## Agent SBOM Generation

Generate a Software Bill of Materials for deployed agents:

```bash
forjar agent-sbom -f agents.yaml --json
```

Output includes:
- All MCP server packages and versions
- Model files with BLAKE3 hashes
- System dependencies (CUDA, Python, Node.js)
- Configuration file checksums
- Tool permission policies

## Agent Recipe Registry

Browse and deploy curated agent recipes:

```bash
# List available agent recipes
forjar agent-registry --category ml-ops

# Deploy a recipe
forjar recipe apply --recipe agent-deployment --inputs model=llama2
```

Recipes are composable — combine GPU setup, model download, MCP server configuration, and health monitoring into a single declarative deployment.

## Cookbook: Complete Agent Deployment

A full agent deployment from bare metal to running service:

```yaml
# agent-full-stack.yaml
machines:
  - name: agent-host
    host: 10.0.1.1
    transport: ssh

data:
  - name: model-version
    type: command
    command: curl -s https://api.example.com/latest-model

resources:
  # Infrastructure
  - name: gpu-driver
    type: gpu_driver
    gpu_backend: nvidia
    version: "550.54.14"

  - name: cuda-toolkit
    type: package
    name: cuda-toolkit-12-4
    depends_on: [gpu-driver]

  # Model
  - name: model
    type: model
    format: safetensors
    source: "huggingface://org/{{data.model-version}}"
    path: /models/latest
    depends_on: [cuda-toolkit]

  # MCP Servers
  - name: inference-server
    type: mcp_server
    package: custom-inference
    config:
      model_path: /models/latest
      gpu: true
    depends_on: [model]

  - name: filesystem-server
    type: mcp_server
    package: "@anthropic/mcp-server-filesystem"
    config:
      allowed_paths: ["/data"]

  # Agent
  - name: agent-config
    type: file
    path: /etc/pforge/config.yaml
    content: |
      name: production-agent
      model: claude-sonnet-4-6
      mcp_servers:
        - name: inference
          url: http://localhost:3001
        - name: filesystem
          command: npx @anthropic/mcp-server-filesystem /data

  - name: agent-service
    type: service
    name: pforge-agent
    command: pforge serve --config /etc/pforge/config.yaml
    depends_on: [agent-config, inference-server, filesystem-server]
    ensure: running

  # Health check
  - name: health-check
    type: command
    command: curl -sf http://localhost:8080/health
    depends_on: [agent-service]
```

Apply with: `forjar apply -f agent-full-stack.yaml --progress`
