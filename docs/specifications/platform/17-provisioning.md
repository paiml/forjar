# 17: Provisioning and Image Generation

> Spec IDs: FJ-33, FJ-49, FJ-51, FJ-52, FJ-54 | Status: IMPLEMENTED

Zero-touch machine provisioning: from bare metal to fully converged in a single boot. This spec covers cross-machine builds, machine bootstrap, cargo binary caching, bootable ISO generation, and Android image packaging.

---

## FJ-33: Build Resource Type (Cross-Compile Build-Deploy)

### Problem

Many deployments require cross-compilation: build on a powerful x86 machine, deploy to an ARM edge device (Jetson, Raspberry Pi). Existing resource types execute on a single machine.

### Design

The `build` resource type introduces a two-machine workflow:

```yaml
apr-binary:
  type: build
  machine: jetson           # deploy target
  build_machine: intel      # where compilation runs
  command: "cargo build --release --target aarch64-unknown-linux-gnu -p apr-cli"
  working_dir: ~/src/aprender
  source: /tmp/cross/release/apr    # artifact path on build machine
  target: ~/.cargo/bin/apr          # deploy path on target machine
  completion_check: "apr --version"
```

### Execution Model

The generated script runs on the **deploy machine** and orchestrates remotely:

1. **Phase 1 (Build)**: SSH to `build_machine`, execute `command` in `working_dir`
2. **Phase 2 (Transfer)**: SCP artifact from `build_machine:source` to `target`
3. **Phase 3 (Verify)**: Run `completion_check` locally on deploy machine

When `build_machine: localhost`, phases 1-2 use local execution (`cp` instead of SSH/SCP).

### Validation

Required fields: `build_machine`, `command`, `source`, `target`. Validated by `validate_build()` in the parser. Missing fields produce clear error messages.

### Codegen Dispatch

| Function | Behavior |
|----------|----------|
| `check_script` | Uses `completion_check` if set, else `test -x <target>` |
| `apply_script` | Three-phase SSH+SCP pipeline |
| `state_query_script` | `sha256sum` of deployed artifact for drift detection |

### Planner Integration

- Default state: `present`
- Proof obligation: Convergent
- Reversibility: Reversible (delete deployed artifact)
- Graph cost: 5 (same as Package)

---

## FJ-49: Bootstrap Command

### Problem

New bare-metal machines require manual SSH key setup, sudo configuration, and connectivity verification before `forjar apply` can manage them.

### Design

```bash
forjar bootstrap -f forjar.yaml --machine yoga [--password]
```

Steps:
1. **SSH key injection**: Copy public key via `ssh-copy-id` (uses `sshpass` if `--password` provided)
2. **Sudo configuration**: Write passwordless sudo rule to `/etc/sudoers.d/<user>-nopasswd`
3. **Verification**: Confirm key-based auth works AND `sudo -n true` succeeds

### Preconditions

- Machine must be defined in `forjar.yaml` with `hostname`, `addr`, `user`
- `ssh_key` field specifies the identity file (`.pub` suffix auto-appended)
- Target must have `sshd` running

### Integration with Image Command

`forjar image` (FJ-52) generates ISOs that pre-configure SSH keys and sudo, eliminating the need for bootstrap on freshly provisioned machines.

---

## FJ-51: Cargo Binary Cache

### Problem

`cargo install` recompiles from source every time, even when the binary is identical. On edge devices or CI, this wastes minutes per package.

### Design

Cache compiled binaries at `~/.forjar/cache/cargo/<pkg>-<version>-<arch>/bin/`:

```
~/.forjar/cache/cargo/
  ripgrep-14.1.0-x86_64/bin/rg
  bat-0.24.0-aarch64/bin/bat
  apr-cli-latest-x86_64/bin/apr
```

### Cache Key

`<package>-<version|"latest">-$(uname -m)`

Architecture is included to prevent cross-arch cache poisoning (e.g., x86 binary cached on aarch64 machine).

### Workflow

```
cache check → HIT:  cp from cache → done
            → MISS: cargo install --root $STAGING
                    → populate cache
                    → cp to $CARGO_HOME/bin
                    → clean staging
```

### Environment Controls

| Variable | Effect |
|----------|--------|
| `FORJAR_CACHE_DIR` | Override cache root (default: `$HOME/.forjar/cache/cargo`) |
| `FORJAR_NO_CARGO_CACHE` | Disable caching entirely (force fresh build) |

### Scope

- Crate installs via `cargo install` are cached
- Source/path installs (`cargo install --path ...`) bypass caching
- `--force --locked --root` flags ensure reproducible builds

---

## FJ-52: Autoinstall ISO Generation

### Problem

Provisioning bare metal requires 20+ minutes of manual Ubuntu installation. For fleet reprovisioning, this doesn't scale.

### Design

`forjar image` generates bootable Ubuntu autoinstall ISOs from `forjar.yaml`:

```bash
# Generate user-data only (for PXE or manual ISO build)
forjar image --user-data -f forjar.yaml -m yoga

# Generate bootable ISO
forjar image --base ubuntu-22.04-live-server-amd64.iso \
  -f forjar.yaml -m yoga -o yoga-autoinstall.iso
```

### User-Data Generation

Reads the `machines:` section and produces Ubuntu autoinstall YAML:

| Machine Field | Autoinstall Field |
|---------------|-------------------|
| `hostname` | `identity.hostname` |
| `user` | `identity.username` |
| `ssh_key` | `ssh.authorized-keys` (reads `.pub` file) |
| `addr` | Static IP comment (manual netplan) |

Additional CLI flags: `--disk` (auto-lvm, auto-zfs, /dev/path), `--locale`, `--timezone`.

### Late Commands

The generated user-data includes:
1. Passwordless sudo for the configured user
2. Copy forjar binary to `/usr/local/bin/forjar`
3. Copy `forjar.yaml` to `/etc/forjar/forjar.yaml`
4. Install `forjar-firstboot.service` (systemd oneshot)

### First-Boot Convergence

```ini
[Unit]
Description=Forjar First Boot Convergence
After=network-online.target
ConditionPathExists=!/etc/forjar/.firstboot-done

[Service]
Type=oneshot
ExecStart=/usr/local/bin/forjar apply --yes -f /etc/forjar/forjar.yaml
ExecStartPost=/usr/bin/touch /etc/forjar/.firstboot-done
TimeoutSec=1800
```

Idempotent: runs once, creates marker file, never runs again.

### ISO Repacking

When `--base` is provided, the command:
1. Extracts base ISO with `xorriso`
2. Injects user-data into `nocloud/` directory
3. Embeds forjar binary and config
4. Repacks with xorriso (preserves UEFI + legacy boot)

Runtime dependency: `xorriso` (detected, error message suggests install).

### Machine Resolution

- Single machine in config: auto-selected
- Multiple machines: `--machine` flag required (error lists available names)

---

## FJ-54: Android Image Generation

### Problem

Android devices (Pixel, Samsung) have powerful NPU/GPU hardware suitable for edge inference, but lack IaC management tooling.

### Design

`forjar image --android` generates a Magisk module ZIP:

```bash
forjar image --android -f forjar.yaml -m pixel -o forjar-magisk.zip
```

### Magisk Module Structure

```
forjar-magisk.zip/
  module.prop              # Module metadata (id, name, version, minMagisk)
  customize.sh             # Installer: set permissions, create dirs
  post-fs-data.sh          # Early boot: create /data/forjar
  service.sh               # Boot service: run forjar apply (idempotent)
  system/
    bin/forjar             # Binary stub (cross-compile separately)
    etc/init/forjar.rc     # Android init.rc service stanza
    etc/forjar/forjar.yaml # Embedded configuration
```

### Boot-Time Convergence

`service.sh` runs at `sys.boot_completed=1`:
- Checks for `/data/forjar/.firstboot-done` marker
- Runs `forjar apply --yes -f <config>`
- Creates marker on success (idempotent)

### init.rc Integration

```
service forjar /system/bin/forjar apply --yes -f /system/etc/forjar/forjar.yaml
    class late_start
    user root
    group root
    oneshot
    disabled

on property:sys.boot_completed=1
    start forjar
```

### Cross-Compilation

The generated ZIP includes a binary stub. Users must cross-compile:

```bash
cargo build --release --target aarch64-linux-android
# Replace stub with real binary before flashing
```

### Limitations

- Experimental (P2 priority)
- Requires rooted device with Magisk
- Binary stub must be replaced with cross-compiled binary
- No `adb` transport yet (SSH via Termux is the current path)

---

## Cross-Machine Dependency Analysis (FJ-1424)

```bash
forjar cross-deps -f forjar.yaml [--json]
```

Analyzes resource dependencies that span machine boundaries. Reports:
- Cross-machine edges (from_resource@machine_a depends_on to_resource@machine_b)
- Execution wave ordering
- Potential bottlenecks in multi-machine convergence

Used by `forjar graph --cross` for visualization and by the planner for wave scheduling.

---

## Integration Map

```
Bootstrap (FJ-49)          Image (FJ-52/54)
    │                           │
    │  SSH key + sudo           │  Autoinstall ISO / Magisk
    │                           │
    ▼                           ▼
Machine ready ──────────► forjar apply
                               │
                               ├── Build (FJ-33): cross-compile + deploy
                               ├── Package (FJ-51): cargo cache acceleration
                               └── All other resource types
```

The provisioning pipeline: `image` pre-configures machines → `bootstrap` handles existing machines → `apply` converges to desired state → `build` enables cross-compilation workflows → `cargo cache` accelerates repeated deployments.
