# Agent Infrastructure & pforge Integration

Forjar manages AI agent infrastructure using its standard resource types: `package` for
dependencies, `file` for configuration, `service` for processes, `model` for ML artifacts,
`gpu` for hardware, and `task` for orchestration steps.

## Architecture

Forjar converges the infrastructure layer. Agent tools compose on top:

```
batuta (AgentOps)    ─── agent lifecycle, tool routing, dispatch
apr-cli (LLMOps)     ─── model pull/convert/compile/serve
pforge               ─── MCP server framework, tool registry
  ↓ all call ↓
forjar               ─── converge packages, files, services, GPU, models
```

Forjar does NOT manage agent logic, tool permissions, or MCP protocol details.
Those are pforge/batuta responsibilities. Forjar ensures the infrastructure converges.

## MCP Server Deployment

Deploy MCP servers as standard forjar resources:

```yaml
version: "1.0"
name: mcp-servers

machines:
  agent-host:
    hostname: agent-01
    addr: 10.0.0.50

resources:
  # Install Node.js for MCP servers
  node-runtime:
    type: package
    machine: agent-host
    provider: apt
    packages: [nodejs, npm]

  # Deploy MCP server config
  mcp-config:
    type: file
    machine: agent-host
    path: /etc/pforge/mcp-servers.json
    content: |
      {
        "servers": [
          {"name": "filesystem", "command": "npx @anthropic/mcp-server-filesystem /data"},
          {"name": "sqlite", "command": "npx @anthropic/mcp-server-sqlite /data/app.db"}
        ]
      }
    mode: "0644"
    depends_on: [node-runtime]

  # pforge agent service
  pforge-service:
    type: service
    machine: agent-host
    name: pforge-agent
    state: running
    enabled: true
    restart_on: [mcp-config]
    depends_on: [mcp-config]
```

## pforge Configuration via File Resources

Deploy pforge agent configurations as file resources:

```yaml
resources:
  pforge-config:
    type: file
    machine: agent-host
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
    mode: "0600"
    owner: pforge
```

## GPU-Accelerated Agent with Local Model

A complete stack: GPU drivers, local model, and inference agent:

```yaml
version: "1.0"
name: gpu-agent

machines:
  gpu-node:
    hostname: gpu-01
    addr: 10.0.0.50

resources:
  # GPU infrastructure
  gpu-setup:
    type: gpu
    machine: gpu-node
    gpu_backend: nvidia
    driver_version: "550"
    cuda_version: "12.4"

  # Download and verify model
  llama-model:
    type: model
    machine: gpu-node
    name: llama-3.2-1b
    source: /models/llama-3.2-1b.gguf
    format: gguf
    quantization: q4_k_m
    checksum: "blake3:a1b2c3..."
    depends_on: [gpu-setup]

  # Install pforge
  pforge-pkg:
    type: package
    machine: gpu-node
    provider: cargo
    packages: [pforge-runtime]

  # Agent config pointing to local model
  agent-config:
    type: file
    machine: gpu-node
    path: /etc/pforge/agent.yaml
    content: |
      name: local-inference-agent
      model_path: /models/llama-3.2-1b.gguf
      gpu: true
    depends_on: [llama-model]

  # Run agent as service
  agent-service:
    type: service
    machine: gpu-node
    name: pforge-agent
    state: running
    enabled: true
    restart_on: [agent-config, llama-model]
    depends_on: [pforge-pkg, agent-config]

  # Health verification
  health-check:
    type: task
    machine: gpu-node
    command: "curl -sf http://localhost:8080/health"
    depends_on: [agent-service]
    timeout: 30
```

## Multi-Agent Fleet

Deploy agent configurations across multiple machines:

```yaml
version: "1.0"
name: agent-fleet

machines:
  agent-01:
    hostname: agent-01
    addr: 10.0.1.1
  agent-02:
    hostname: agent-02
    addr: 10.0.1.2
  agent-03:
    hostname: agent-03
    addr: 10.0.1.3

resources:
  # Deploy config to ALL agents
  agent-config:
    type: file
    machine: [agent-01, agent-02, agent-03]
    path: /etc/pforge/config.yaml
    content: |
      name: fleet-agent
      model: claude-sonnet-4-6
    mode: "0600"

  # Start service on ALL agents
  agent-service:
    type: service
    machine: [agent-01, agent-02, agent-03]
    name: pforge-agent
    state: running
    enabled: true
    restart_on: [agent-config]
    depends_on: [agent-config]

policy:
  parallel_machines: true
  serial: 1            # Rolling deploy: 1 machine at a time
  max_fail_percentage: 33
```

## Agent Health Monitoring

Monitor agent fleet health using forjar's built-in commands:

```bash
# Check agent service status across fleet
forjar status -f agent-fleet.yaml

# Detect drifted configurations
forjar drift -f agent-fleet.yaml

# Plan and verify before rolling out config changes
forjar plan -f agent-fleet.yaml
forjar apply -f agent-fleet.yaml
```

Drift detection catches:
- Configuration file modifications (BLAKE3 hash mismatch)
- Model file corruption or unauthorized changes
- Service stopped or disabled
- Package version changes

## batuta Integration

batuta (AgentOps) orchestrates agent lifecycle on forjar-converged infrastructure:

```bash
# batuta workflow:
forjar apply -f agent-infra.yaml       # converge infrastructure
batuta deploy --agent ops-agent        # register agent with batuta
batuta dispatch ops-agent --task "analyze logs" --param date=today
batuta status --fleet                  # monitor agent fleet
```

Forjar owns infrastructure convergence. batuta owns agent dispatch, tool routing,
and coordination. The boundary is clean: forjar ensures files, services, and models
are in the desired state; batuta decides what the agent does.

## Agent Recipe

Use forjar recipes for reusable agent deployment patterns:

```yaml
# recipes/pforge-agent.yaml
recipe:
  name: pforge-agent
  version: "1.0"
  description: Deploy a pforge MCP agent
  inputs:
    agent_name:
      type: string
      required: true
    model:
      type: string
      default: "claude-sonnet-4-6"
    port:
      type: string
      default: "8080"

resources:
  config:
    type: file
    path: "/etc/pforge/{{inputs.agent_name}}.yaml"
    content: |
      name: {{inputs.agent_name}}
      model: {{inputs.model}}
      port: {{inputs.port}}

  service:
    type: service
    name: "pforge-{{inputs.agent_name}}"
    state: running
    enabled: true
    restart_on: [config]
    depends_on: [config]
```

Use the recipe:

```yaml
resources:
  my-agent:
    type: recipe
    recipe: recipes/pforge-agent.yaml
    inputs:
      agent_name: ops-agent
      model: claude-sonnet-4-6
      port: "8080"
```
