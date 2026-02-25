# Resource Types

## Package

Install or remove system packages.

```yaml
resources:
  dev-tools:
    type: package
    machine: m1
    provider: apt          # apt | cargo | uv
    packages: [curl, git, htop]
    state: present         # present (default) | absent
    version: "1.2.3"       # optional version pin
```

### Providers

| Provider | Install Command | Version Syntax | Remove Command |
|----------|----------------|----------------|----------------|
| `apt` | `apt-get install -y` (auto-sudo if non-root) | `package=version` | `apt-get remove -y` |
| `cargo` | `cargo install --force` | `package@version` | — |
| `uv` | `uv tool install --force` | `package==version` | `uv tool uninstall` |

### Version Pinning

Pin all packages in a resource to a specific version:

```yaml
  nginx:
    type: package
    machine: web-server
    provider: apt
    packages: [nginx]
    version: "1.18.0-0ubuntu1"
```

## File

Manage files, directories, and symlinks.

### Regular File

```yaml
resources:
  config:
    type: file
    machine: m1
    path: /etc/app/config.yaml
    content: |
      database:
        host: localhost
        port: 5432
    owner: app
    group: app
    mode: "0640"
```

Content is written via heredoc (`<<'FORJAR_EOF'`) — shell variable expansion is prevented.

### Source File Transfer

Instead of inline content, use `source` to transfer a local file:

```yaml
resources:
  entrypoint:
    type: file
    machine: m1
    path: /opt/app/entrypoint.sh
    source: scripts/entrypoint.sh    # local path, read at apply time
    owner: app
    mode: "0755"
```

The file is base64-encoded locally and decoded on the remote machine via `base64 -d`. This works with all transports (local, SSH, container) and handles binary files safely.

`content` and `source` are mutually exclusive — use one or the other.

### Directory

```yaml
resources:
  data-dir:
    type: file
    machine: m1
    state: directory
    path: /var/lib/app/data
    owner: app
    mode: "0755"
```

Creates the directory (and parents) with `mkdir -p`.

### Symlink

```yaml
resources:
  tool-link:
    type: file
    machine: m1
    state: symlink
    path: /usr/local/bin/tool
    target: /opt/tool/bin/tool
```

### Absent (Delete)

```yaml
resources:
  old-config:
    type: file
    machine: m1
    state: absent
    path: /etc/old-app.conf
```

Removes the file or directory with `rm -rf`.

### File Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | string | required | Absolute file path |
| `content` | string | — | Inline file content (mutually exclusive with source) |
| `source` | string | — | Local file path to transfer (mutually exclusive with content) |
| `state` | string | `file` | file, directory, symlink, absent |
| `target` | string | — | Symlink target (state=symlink only) |
| `owner` | string | — | File owner |
| `group` | string | — | File group |
| `mode` | string | — | Octal permissions (e.g. "0644") |

## Service

Manage systemd services. Includes automatic systemd detection — if `systemctl` is not available (e.g. inside containers without systemd), service resources are gracefully skipped with a warning rather than failing.

```yaml
resources:
  nginx:
    type: service
    machine: m1
    name: nginx
    state: running         # running | stopped
    enabled: true          # Enable on boot
    restart_on: [config]   # Restart when these resources change
```

### Service States

| State | Action |
|-------|--------|
| `running` | `systemctl start` + optionally `systemctl enable` |
| `stopped` | `systemctl stop` + optionally `systemctl disable` |
| `enabled` | `systemctl enable` (no start/stop) |
| `disabled` | `systemctl disable` (no start/stop) |

### Restart Triggers

Use `restart_on` to restart a service when a dependency changes:

```yaml
resources:
  nginx-conf:
    type: file
    machine: m1
    path: /etc/nginx/nginx.conf
    content: "..."

  nginx:
    type: service
    machine: m1
    name: nginx
    state: running
    restart_on: [nginx-conf]
    depends_on: [nginx-conf]
```

## Mount

Manage filesystem mounts.

```yaml
resources:
  data-mount:
    type: mount
    machine: m1
    source: /dev/sdb1
    path: /mnt/data
    fstype: ext4
    options: "defaults,noatime"
    state: mounted         # mounted | unmounted | absent
```

### NFS Mount

```yaml
resources:
  nfs-share:
    type: mount
    machine: m1
    source: "192.168.1.10:/exports/data"
    path: /mnt/nfs
    fstype: nfs
    options: "rw,soft,intr"
    state: mounted
```

### Mount Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | string | required | Device or NFS path |
| `path` | string | required | Mount point |
| `fstype` | string | — | Filesystem type (ext4, nfs, etc.) |
| `options` | string | — | Mount options |
| `state` | string | `mounted` | mounted, unmounted, absent |

## User

Manage local system users and groups via `useradd`/`usermod`/`userdel`.

```yaml
resources:
  deploy-user:
    type: user
    machine: m1
    name: deploy
    shell: /bin/bash
    home: /home/deploy
    groups: [docker, sudo]
    ssh_authorized_keys:
      - "ssh-ed25519 AAAA... deploy@workstation"
```

### System Users

```yaml
resources:
  prometheus:
    type: user
    machine: m1
    name: prometheus
    system_user: true
    shell: /usr/sbin/nologin
```

System users are created with `--system` and do not get a home directory by default.

### Remove a User

```yaml
resources:
  old-user:
    type: user
    machine: m1
    name: olduser
    state: absent
```

### User Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Username |
| `state` | string | `present` | present, absent |
| `uid` | integer | — | Explicit UID |
| `group` | string | — | Primary group (--gid) |
| `groups` | [string] | [] | Supplementary groups (auto-created if missing) |
| `shell` | string | — | Login shell |
| `home` | string | `/home/{name}` | Home directory |
| `system_user` | bool | false | Create as system user (--system) |
| `ssh_authorized_keys` | [string] | [] | SSH public keys for ~/.ssh/authorized_keys |

## Docker

Manage Docker containers as deployed resources. This is distinct from container *transport* (using containers as execution targets) — this manages containers running ON machines.

```yaml
resources:
  web:
    type: docker
    machine: m1
    name: web
    image: nginx:latest
    state: running
    ports: ["8080:80", "443:443"]
    volumes: ["/data/web:/usr/share/nginx/html"]
    environment: ["NGINX_HOST=example.com"]
    restart: unless-stopped
```

### Docker States

| State | Action |
|-------|--------|
| `running` | Pull image, stop/remove existing, `docker run -d` |
| `stopped` | `docker stop` |
| `absent` | `docker stop` + `docker rm` |

### Docker Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Container name |
| `image` | string | required | OCI image (e.g. `nginx:latest`) |
| `state` | string | `running` | running, stopped, absent |
| `ports` | [string] | [] | Port mappings (`host:container`) |
| `volumes` | [string] | [] | Volume mounts (`host:container`) |
| `environment` | [string] | [] | Environment variables (`KEY=VALUE`) |
| `restart` | string | — | Restart policy (no, always, unless-stopped, on-failure) |
| `command` | string | — | Override container command |

## Cron

Manage scheduled tasks via crontab entries. Jobs are tagged with `# forjar:{name}` comments for idempotent updates.

```yaml
resources:
  backup:
    type: cron
    machine: m1
    name: nightly-backup
    schedule: "0 2 * * *"
    command: /usr/local/bin/backup.sh
    owner: root
```

### Remove a Cron Job

```yaml
resources:
  old-job:
    type: cron
    machine: m1
    name: old-job
    state: absent
    owner: root
```

### Cron Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Job identifier (used as tag in crontab) |
| `state` | string | `present` | present, absent |
| `schedule` | string | `* * * * *` | Cron schedule expression |
| `command` | string | required | Command to execute |
| `owner` | string | `root` | Crontab user |

## Network

Manage firewall rules via ufw (Uncomplicated Firewall).

```yaml
resources:
  allow-ssh:
    type: network
    machine: m1
    name: ssh-access
    port: "22"
    protocol: tcp
    action: allow
    from_addr: 192.168.1.0/24
```

### Network States

| State | Action |
|-------|--------|
| `present` | Add ufw rule (enables ufw if not active) |
| `absent` | `ufw delete` the rule |

### Network Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | — | Rule comment |
| `state` | string | `present` | present, absent |
| `port` | string | required | Port number |
| `protocol` | string | `tcp` | tcp, udp |
| `action` | string | `allow` | allow, deny, reject |
| `from_addr` | string | — | Source address/CIDR (e.g. `192.168.1.0/24`) |
