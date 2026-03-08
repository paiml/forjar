# Provisioning & Image Generation

Zero-touch machine provisioning: from bare metal to fully converged in a single boot.

## Build Resource Type (FJ-33)

The `build` resource type introduces a two-machine workflow. Compile on a powerful build machine, transfer the artifact, and deploy to a target device.

```yaml
resources:
  apr-binary:
    type: build
    machine: jetson           # deploy target
    build_machine: intel      # where compilation runs
    command: "cargo build --release --target aarch64-unknown-linux-gnu -p apr-cli"
    working_dir: ~/src/aprender
    source: /tmp/cross/release/apr
    target: ~/.cargo/bin/apr
    completion_check: "apr --version"
```

### Execution phases

1. **Build**: SSH to `build_machine`, execute `command` in `working_dir`
2. **Transfer**: SCP artifact from `build_machine:source` to `target`
3. **Verify**: Run `completion_check` locally on deploy machine

When `build_machine: localhost`, local `cp` replaces SSH/SCP.

### Generated scripts

```bash
# Check script (uses completion_check)
if apr --version >/dev/null 2>&1; then echo 'installed:build'; else echo 'missing:build'; fi

# Apply script (three-phase pipeline)
set -euo pipefail
ssh -o BatchMode=yes intel 'cd ~/src/aprender && cargo build --release ...'
scp -o BatchMode=yes 'intel:/tmp/cross/release/apr' '~/.cargo/bin/apr'
apr --version

# State query (sha256sum for drift detection)
sha256sum '~/.cargo/bin/apr' | awk '{print $1}'
```

Try it: `cargo run --example build_resource`

## Bootstrap Command (FJ-49)

Prepare a bare-metal machine for forjar management:

```bash
forjar bootstrap -f forjar.yaml --machine yoga
forjar bootstrap -f forjar.yaml --machine yoga --password
```

### Phases

| Phase | Action | Verification |
|-------|--------|--------------|
| 1 | Copy SSH public key via `ssh-copy-id` | `ssh -o BatchMode=yes user@host true` |
| 2 | Write sudoers rule to `/etc/sudoers.d/` | `sudo -n true` |
| 3 | Set hostname (optional) | `hostnamectl` |

With `--password`, uses `sshpass` for non-interactive key injection.

Machines provisioned via `forjar image` skip bootstrap entirely.

Try it: `cargo run --example bootstrap_machine`

## Cargo Binary Cache (FJ-51)

Cargo packages managed by forjar use an architecture-aware binary cache:

```
~/.forjar/cache/cargo/
  ripgrep-14.1.0-x86_64/bin/rg
  bat-0.24.0-aarch64/bin/bat
```

Cache key: `<package>-<version>-$(uname -m)`

```
cache check → HIT:  cp from cache
            → MISS: cargo install → populate cache → cp to $CARGO_HOME/bin
```

| Variable | Effect |
|----------|--------|
| `FORJAR_CACHE_DIR` | Override cache root |
| `FORJAR_NO_CARGO_CACHE` | Disable caching entirely |

## Autoinstall ISO Generation (FJ-52)

Generate bootable Ubuntu autoinstall ISOs from `forjar.yaml`:

```bash
# User-data only (for PXE or manual ISO build)
forjar image --user-data -f forjar.yaml -m yoga

# Bootable ISO (requires xorriso)
forjar image --base ubuntu-22.04-live-server-amd64.iso \
  -f forjar.yaml -m yoga -o yoga-autoinstall.iso
```

### What gets generated

The autoinstall user-data includes:

- **Identity**: hostname, username from machine config
- **SSH**: authorized keys from `ssh_key` field
- **Storage**: LVM, ZFS, or explicit disk path (`--disk`)
- **Sudo**: passwordless sudo for the configured user
- **Firstboot service**: systemd oneshot that runs `forjar apply` on first boot

### First-boot convergence

```ini
[Service]
Type=oneshot
ExecStart=/usr/local/bin/forjar apply --yes -f /etc/forjar/forjar.yaml
ExecStartPost=/usr/bin/touch /etc/forjar/.firstboot-done
```

The ISO embeds both the forjar binary and configuration. On first boot, the machine converges to desired state automatically.

Try it: `cargo run --example image_generation`

## Android Image Generation (FJ-54)

Generate a Magisk module ZIP for rooted Android devices:

```bash
forjar image --android -f forjar.yaml -m pixel -o forjar-magisk.zip
```

### Module contents

| File | Purpose |
|------|---------|
| `module.prop` | Magisk metadata |
| `customize.sh` | Set permissions during install |
| `post-fs-data.sh` | Create `/data/forjar` at early boot |
| `service.sh` | Run `forjar apply` at `boot_completed` |
| `system/bin/forjar` | Binary stub (replace with cross-compiled) |
| `system/etc/init/forjar.rc` | Android init service stanza |
| `system/etc/forjar/forjar.yaml` | Embedded configuration |

### Cross-compilation

```bash
cargo build --release --target aarch64-linux-android
# Replace the stub in the ZIP with the real binary
```

> **Note**: Experimental. Requires rooted device with Magisk 25+.

## Cross-Machine Dependency Analysis (FJ-1424)

```bash
forjar cross-deps -f forjar.yaml [--json]
```

Analyzes resource dependencies that span machine boundaries:
- Cross-machine edges (resource@machine_a depends_on resource@machine_b)
- Execution wave ordering
- Bottleneck identification in multi-machine convergence

## Provisioning Pipeline

```
Bootstrap (FJ-49)          Image (FJ-52/54)
    │                           │
    │  SSH key + sudo           │  Autoinstall ISO / Magisk
    │                           │
    ▼                           ▼
Machine ready ──────────► forjar apply
                               │
                               ├── Build (FJ-33): cross-compile + deploy
                               ├── Package (FJ-51): cargo cache
                               └── All other resource types
```

**Path 1** (existing machines): `bootstrap` → `apply`

**Path 2** (new machines): `image` → boot → firstboot `apply`

Both paths converge to the same state. The image path is fully automated; the bootstrap path handles machines that already have an OS installed.
