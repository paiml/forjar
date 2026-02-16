# Resource Types

## Package

Install or remove system packages.

```yaml
resources:
  dev-tools:
    type: package
    machine: m1
    provider: apt          # apt | cargo | pip
    packages: [curl, git, htop]
    state: present         # present (default) | absent
```

### Providers

| Provider | Install Command | Remove Command |
|----------|----------------|----------------|
| `apt` | `apt-get install -y` | `apt-get remove -y` |
| `cargo` | `cargo install` | `cargo uninstall` |
| `pip` | `pip install` | `pip uninstall -y` |

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
| `content` | string | — | Inline file content |
| `state` | string | `file` | file, directory, symlink, absent |
| `target` | string | — | Symlink target (state=symlink only) |
| `owner` | string | — | File owner |
| `group` | string | — | File group |
| `mode` | string | — | Octal permissions (e.g. "0644") |

## Service

Manage systemd services.

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
    path: /mnt/data
    target: /dev/sdb1
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
    path: /mnt/nfs
    target: "192.168.1.10:/exports/data"
    fstype: nfs
    options: "rw,soft,intr"
    state: mounted
```

### Mount Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | string | required | Mount point |
| `target` | string | required | Device or NFS path |
| `fstype` | string | — | Filesystem type (ext4, nfs, etc.) |
| `options` | string | — | Mount options |
| `state` | string | `mounted` | mounted, unmounted, absent |
