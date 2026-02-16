# Cookbook

Real-world configuration examples.

## Home Lab GPU Server

```yaml
version: "1.0"
name: home-lab
description: "Sovereign AI development environment"

params:
  data_dir: /mnt/nvme-raid0/data
  user: noah

machines:
  gpu-box:
    hostname: lambda
    addr: 192.168.50.100
    user: noah
    ssh_key: ~/.ssh/id_ed25519

resources:
  # Development tools
  dev-packages:
    type: package
    machine: gpu-box
    provider: apt
    packages:
      - build-essential
      - cmake
      - curl
      - git
      - htop
      - jq
      - ripgrep
      - tmux
      - vim

  # Data directory
  data-dir:
    type: file
    machine: gpu-box
    state: directory
    path: "{{params.data_dir}}"
    owner: "{{params.user}}"
    mode: "0755"
    depends_on: [dev-packages]

  # Git config
  gitconfig:
    type: file
    machine: gpu-box
    path: "/home/{{params.user}}/.gitconfig"
    content: |
      [user]
        name = Noah Gift
        email = noah@example.com
      [core]
        editor = vim
      [pull]
        rebase = true
    owner: "{{params.user}}"
    mode: "0644"

policy:
  failure: stop_on_first
  tripwire: true
```

## Multi-Machine Web Stack

```yaml
version: "1.0"
name: web-stack
description: "Web application with load balancer"

params:
  app_version: "2.1.0"
  domain: example.com

machines:
  lb:
    hostname: lb1
    addr: 10.0.0.10
    user: deploy
  web1:
    hostname: web1
    addr: 10.0.0.11
    user: deploy
  web2:
    hostname: web2
    addr: 10.0.0.12
    user: deploy

resources:
  # Install nginx on all web servers
  nginx-pkg:
    type: package
    machine: [web1, web2]
    provider: apt
    packages: [nginx]

  # App config (templated)
  app-config:
    type: file
    machine: [web1, web2]
    path: /etc/app/config.yaml
    content: |
      version: {{params.app_version}}
      domain: {{params.domain}}
      listen: 0.0.0.0:8080
    owner: deploy
    mode: "0640"
    depends_on: [nginx-pkg]

  # Nginx service
  nginx-svc:
    type: service
    machine: [web1, web2]
    name: nginx
    state: running
    enabled: true
    restart_on: [app-config]
    depends_on: [app-config]

  # HAProxy on load balancer
  haproxy:
    type: package
    machine: lb
    provider: apt
    packages: [haproxy]
```

## Edge Device Fleet

```yaml
version: "1.0"
name: edge-fleet
description: "Jetson Orin fleet provisioning"

params:
  model_version: "v3.2"

machines:
  jetson-1:
    hostname: jetson-edge-1
    addr: 192.168.55.1
    user: nvidia
    arch: aarch64
  jetson-2:
    hostname: jetson-edge-2
    addr: 192.168.55.2
    user: nvidia
    arch: aarch64

resources:
  base:
    type: package
    machine: [jetson-1, jetson-2]
    provider: apt
    packages: [curl, htop, python3-pip]

  model-dir:
    type: file
    machine: [jetson-1, jetson-2]
    state: directory
    path: /opt/models
    owner: nvidia
    mode: "0755"
    depends_on: [base]

  inference-config:
    type: file
    machine: [jetson-1, jetson-2]
    path: /opt/models/config.yaml
    content: |
      model_version: {{params.model_version}}
      device: cuda
      batch_size: 1
    owner: nvidia
    mode: "0644"
    depends_on: [model-dir]
```

## Removing Old Resources

Use `state: absent` to clean up:

```yaml
resources:
  old-config:
    type: file
    machine: web1
    state: absent
    path: /etc/old-app/config.yaml

  old-mount:
    type: mount
    machine: web1
    state: absent
    path: /mnt/old-nfs
```

## NFS Data Mount

```yaml
resources:
  nfs-data:
    type: mount
    machine: gpu-box
    path: /mnt/shared
    target: "192.168.1.10:/exports/data"
    fstype: nfs
    options: "rw,soft,intr,timeo=30"
    state: mounted
```
