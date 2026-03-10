# 25: Distribution Artifact Generation

> Generate install scripts, package manifests, and registry metadata for any forjar-managed binary.

**Spec IDs**: FJ-3600 (dist command family) | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md) | **Status**: PROPOSED

---

## Problem

Every CLI tool needs the same 6 distribution artifacts:

1. **Shell installer** (`install.sh`) — `curl -sSf https://example.com/install.sh | sh`
2. **Homebrew formula** — `brew install org/tap/tool`
3. **cargo-binstall metadata** — `cargo binstall tool` (skip compilation)
4. **Nix flake** — `nix run github:org/tool`
5. **GitHub Actions setup** — `uses: org/setup-tool@v1`
6. **OS packages** — `.deb`, `.rpm`, `.apk` spec files

Every project hand-writes these. They drift. They have subtle bugs (wrong checksum, wrong arch detection, missing error handling). Forjar already has every building block: shell codegen, BLAKE3 integrity, OS/arch detection, `github_release` asset resolution, and template expansion.

**Thesis**: Distribution artifacts are infrastructure. Forjar should generate them declaratively from a single `dist:` section, the same way it generates apply scripts from `resources:`.

---

## CLI

```bash
# Generate all distribution artifacts for a project
forjar dist -f forjar.yaml --all

# Generate specific artifacts
forjar dist -f forjar.yaml --installer          # shell installer script
forjar dist -f forjar.yaml --homebrew           # Homebrew formula
forjar dist -f forjar.yaml --binstall           # cargo-binstall metadata
forjar dist -f forjar.yaml --nix                # Nix flake
forjar dist -f forjar.yaml --github-action      # GitHub Actions setup action
forjar dist -f forjar.yaml --deb                # Debian package spec
forjar dist -f forjar.yaml --rpm                # RPM spec file

# Output control
forjar dist -f forjar.yaml --installer --output dist/install.sh
forjar dist -f forjar.yaml --all --output-dir dist/

# Verify generated artifacts (dogfood: run the installer in a sandbox)
forjar dist -f forjar.yaml --verify

# JSON manifest of all generated artifacts
forjar dist -f forjar.yaml --all --json
```

---

## Config Schema

New top-level `dist:` section in forjar.yaml:

```yaml
version: "1.0"
name: forjar
dist:
  # Required: where binaries come from
  source: github_release          # or: local, url, s3
  repo: paiml/forjar              # GitHub org/repo
  binary: forjar                  # binary name after install

  # Targets to build (maps to GitHub Release asset names)
  targets:
    - os: linux
      arch: x86_64
      asset: "forjar-{version}-x86_64-unknown-linux-gnu.tar.gz"
      libc: gnu
    - os: linux
      arch: x86_64
      asset: "forjar-{version}-x86_64-unknown-linux-musl.tar.gz"
      libc: musl
    - os: linux
      arch: aarch64
      asset: "forjar-{version}-aarch64-unknown-linux-gnu.tar.gz"
      libc: gnu
    - os: linux
      arch: aarch64
      asset: "forjar-{version}-aarch64-unknown-linux-musl.tar.gz"
      libc: musl
    - os: darwin
      arch: x86_64
      asset: "forjar-{version}-x86_64-apple-darwin.tar.gz"
    - os: darwin
      arch: aarch64
      asset: "forjar-{version}-aarch64-apple-darwin.tar.gz"

  # Where to install
  install_dir: /usr/local/bin     # default
  install_dir_fallback: ~/.local/bin  # if /usr/local/bin not writable

  # Integrity
  checksums: SHA256SUMS           # asset name for checksum file
  checksum_algo: sha256           # sha256 | blake3

  # Package metadata (used by Homebrew, Nix, deb, rpm)
  description: "Rust-native Infrastructure as Code"
  homepage: https://forjar.dev
  license: "MIT OR Apache-2.0"
  maintainer: "Pragmatic AI Labs"

  # Version resolution
  version_cmd: "forjar --version" # verify after install
  latest_tag: true                # resolve latest GitHub tag

  # Optional: post-install
  post_install: |
    forjar completion --shell bash > /etc/bash_completion.d/forjar 2>/dev/null || true
    forjar completion --shell zsh > "${fpath[1]}/_forjar" 2>/dev/null || true

  # Optional: Homebrew-specific
  homebrew:
    tap: paiml/tap
    dependencies: []              # brew deps
    caveats: |
      To get started, run: forjar init

  # Optional: Nix-specific
  nix:
    inputs:
      nixpkgs: "github:NixOS/nixpkgs/nixos-unstable"
    build_inputs: [ openssl, pkg-config ]
```

---

## Generated Artifacts

### 1. Shell Installer (`--installer`) — FJ-3601

Generates a POSIX-compliant install script that:

```
detect_os()        → linux | darwin
detect_arch()      → x86_64 | aarch64 | arm64
detect_libc()      → gnu | musl (via ldd --version)
resolve_version()  → latest tag or pinned
build_asset_url()  → https://github.com/{repo}/releases/download/{tag}/{asset}
download()         → curl -fsSL (fallback: wget)
verify_checksum()  → sha256sum / shasum -a 256
extract()          → tar xzf
install()          → mv to install_dir (sudo if needed)
verify_install()   → run version_cmd
post_install()     → completions, etc.
```

**Requirements**:
- POSIX sh (not bash) — works on Alpine, BusyBox, minimal containers
- No external dependencies beyond curl/wget, tar, sha256sum/shasum
- Graceful fallback: try `/usr/local/bin`, fall back to `~/.local/bin`, update `$PATH` hint
- `--yes` flag for non-interactive mode
- `--version <tag>` for pinned installs
- `--prefix <dir>` for custom install location
- Color output when terminal, plain when piped
- Error messages include the failing step and remediation

**Anti-patterns explicitly avoided**:
- No `curl | bash` in the generated script itself (it IS the script people curl)
- No eval of downloaded content
- Always verify checksum before extraction
- Never silently overwrite — warn if binary exists, `--force` to replace

**Generated output**:
```sh
#!/bin/sh
# install.sh — generated by forjar dist (do not edit)
# Usage: curl -sSf https://forjar.dev/install.sh | sh
# Pinned: curl -sSf https://forjar.dev/install.sh | sh -s -- --version v1.1.1
set -eu
...
```

### 2. Homebrew Formula (`--homebrew`) — FJ-3602

Generates a Ruby formula for the Homebrew tap:

```ruby
class Forjar < Formula
  desc "Rust-native Infrastructure as Code"
  homepage "https://forjar.dev"
  license "MIT" => { with: "Apache-2.0" }
  version "1.1.1"

  on_linux do
    on_intel do
      url "https://github.com/paiml/forjar/releases/download/v1.1.1/forjar-1.1.1-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "abc123..."
    end
    on_arm do
      url "https://github.com/paiml/forjar/releases/download/v1.1.1/forjar-1.1.1-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "def456..."
    end
  end

  on_macos do
    on_intel do
      url "https://github.com/paiml/forjar/releases/download/v1.1.1/forjar-1.1.1-x86_64-apple-darwin.tar.gz"
      sha256 "ghi789..."
    end
    on_arm do
      url "https://github.com/paiml/forjar/releases/download/v1.1.1/forjar-1.1.1-aarch64-apple-darwin.tar.gz"
      sha256 "jkl012..."
    end
  end

  def install
    bin.install "forjar"
  end

  def post_install
    (bash_completion/"forjar").write Utils.safe_popen_read(bin/"forjar", "completion", "--shell", "bash")
    (zsh_completion/"_forjar").write Utils.safe_popen_read(bin/"forjar", "completion", "--shell", "zsh")
  end

  test do
    assert_match "forjar", shell_output("#{bin}/forjar --version")
  end
end
```

**Requirements**:
- Fetch real SHA256 checksums from GitHub Release `SHA256SUMS` asset
- Platform-conditional URL blocks (on_linux/on_macos × on_intel/on_arm)
- Shell completions in post_install
- Smoke test in test block

### 3. cargo-binstall Metadata (`--binstall`) — FJ-3603

Adds `[package.metadata.binstall]` to Cargo.toml:

```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/v{ version }/{ name }-{ version }-{ target }{ archive-suffix }"
bin-dir = "{ bin }{ binary-ext }"
pkg-fmt = "tgz"

[package.metadata.binstall.overrides.x86_64-unknown-linux-musl]
pkg-url = "{ repo }/releases/download/v{ version }/{ name }-{ version }-x86_64-unknown-linux-musl.tar.gz"

[package.metadata.binstall.overrides.aarch64-unknown-linux-gnu]
pkg-url = "{ repo }/releases/download/v{ version }/{ name }-{ version }-aarch64-unknown-linux-gnu.tar.gz"
```

**Requirements**:
- Output as TOML fragment (user pastes into Cargo.toml) or `--apply` to patch in-place
- Cover all targets from `dist.targets`

### 4. Nix Flake (`--nix`) — FJ-3604

Generates `flake.nix`:

```nix
{
  description = "Rust-native Infrastructure as Code";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        version = "1.1.1";
        src = {
          "x86_64-linux" = pkgs.fetchurl {
            url = "https://github.com/paiml/forjar/releases/download/v${version}/forjar-${version}-x86_64-unknown-linux-gnu.tar.gz";
            sha256 = "abc123...";
          };
          "aarch64-linux" = pkgs.fetchurl {
            url = "https://github.com/paiml/forjar/releases/download/v${version}/forjar-${version}-aarch64-unknown-linux-gnu.tar.gz";
            sha256 = "def456...";
          };
          "x86_64-darwin" = pkgs.fetchurl {
            url = "https://github.com/paiml/forjar/releases/download/v${version}/forjar-${version}-x86_64-apple-darwin.tar.gz";
            sha256 = "ghi789...";
          };
          "aarch64-darwin" = pkgs.fetchurl {
            url = "https://github.com/paiml/forjar/releases/download/v${version}/forjar-${version}-aarch64-apple-darwin.tar.gz";
            sha256 = "jkl012...";
          };
        }.${system} or (throw "unsupported system: ${system}");
      in {
        packages.default = pkgs.stdenv.mkDerivation {
          pname = "forjar";
          inherit version;
          inherit src;
          sourceRoot = ".";
          unpackPhase = "tar xzf $src";
          installPhase = ''
            mkdir -p $out/bin
            cp forjar $out/bin/
          '';
        };
      }
    );
}
```

### 5. GitHub Actions Setup (`--github-action`) — FJ-3605

Generates `action.yml` for a `setup-forjar` action:

```yaml
name: Setup Forjar
description: Install forjar CLI for GitHub Actions
inputs:
  version:
    description: "Version to install (default: latest)"
    required: false
    default: "latest"
runs:
  using: composite
  steps:
    - name: Install forjar
      shell: bash
      run: |
        VERSION="${{ inputs.version }}"
        if [ "$VERSION" = "latest" ]; then
          VERSION=$(curl -sSf https://api.github.com/repos/paiml/forjar/releases/latest | grep tag_name | cut -d'"' -f4)
        fi
        ARCH=$(uname -m)
        OS=$(uname -s | tr '[:upper:]' '[:lower:]')
        case "$ARCH" in
          x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
          aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
        esac
        curl -sSfL "https://github.com/paiml/forjar/releases/download/${VERSION}/forjar-${VERSION#v}-${TARGET}.tar.gz" | tar xz
        sudo mv forjar /usr/local/bin/
        forjar --version
```

### 6. OS Packages (`--deb`, `--rpm`) — FJ-3606

**Debian** — generates `debian/` directory structure:

```
debian/
  control       # Package: forjar, Version, Depends, Description
  rules         # Build rules (just install pre-built binary)
  changelog     # Auto-generated from CHANGELOG.md
  copyright     # MIT OR Apache-2.0
  install       # forjar usr/local/bin
```

**RPM** — generates `.spec` file:

```spec
Name:    forjar
Version: 1.1.1
Release: 1
Summary: Rust-native Infrastructure as Code
License: MIT OR Apache-2.0
URL:     https://forjar.dev
Source0: forjar-%{version}-x86_64-unknown-linux-gnu.tar.gz

%install
mkdir -p %{buildroot}/usr/local/bin
cp forjar %{buildroot}/usr/local/bin/

%files
/usr/local/bin/forjar
```

---

## Version Pinning and Checksum Resolution

When generating artifacts that need real checksums (Homebrew, Nix), forjar must resolve them:

```
fn resolve_checksums(dist_config, version):
    if dist_config.checksums:
        // Download SHA256SUMS from release
        let sums = fetch("{repo}/releases/download/{version}/{checksums}")
        parse_checksum_file(sums)
    else:
        // Download each asset, compute checksum locally
        for target in dist_config.targets:
            let url = build_asset_url(target, version)
            let bytes = fetch(url)
            checksums[target] = sha256(bytes)
```

For `--installer`, checksums are embedded as constants. For `--homebrew` and `--nix`, checksums are embedded per-platform.

When no version is specified, `--latest` resolves the latest GitHub tag via the API.

---

## Verification (`--verify`) — FJ-3607

Dogfood the generated artifacts in a sandbox:

```bash
forjar dist -f forjar.yaml --verify
```

Runs each generated artifact in a container to verify it works:

| Artifact | Verification |
|----------|-------------|
| `install.sh` | Run in `alpine:latest` and `ubuntu:latest` containers |
| Homebrew formula | `brew install --build-from-source` in Homebrew container |
| Nix flake | `nix build` in NixOS container |
| GitHub Action | Dry-run with `act` |
| `.deb` | `dpkg -i` in Debian container |
| `.rpm` | `rpm -i` in Fedora container |

Each verification checks:
1. Install completes without error
2. `binary --version` runs successfully
3. Binary is on `$PATH`
4. Checksum matches expected value

---

## Dogfooding: Forjar Distributes Itself

The first consumer of `forjar dist` is forjar itself:

```yaml
# In forjar's own forjar.yaml
dist:
  source: github_release
  repo: paiml/forjar
  binary: forjar
  targets:
    - { os: linux, arch: x86_64, asset: "forjar-{version}-x86_64-unknown-linux-gnu.tar.gz", libc: gnu }
    - { os: linux, arch: x86_64, asset: "forjar-{version}-x86_64-unknown-linux-musl.tar.gz", libc: musl }
    - { os: linux, arch: aarch64, asset: "forjar-{version}-aarch64-unknown-linux-gnu.tar.gz", libc: gnu }
    - { os: linux, arch: aarch64, asset: "forjar-{version}-aarch64-unknown-linux-musl.tar.gz", libc: musl }
    - { os: darwin, arch: x86_64, asset: "forjar-{version}-x86_64-apple-darwin.tar.gz" }
    - { os: darwin, arch: aarch64, asset: "forjar-{version}-aarch64-apple-darwin.tar.gz" }
  checksums: SHA256SUMS
  install_dir: /usr/local/bin
  description: "Rust-native Infrastructure as Code"
  homepage: https://forjar.dev
  license: "MIT OR Apache-2.0"
  homebrew:
    tap: paiml/tap
  post_install: |
    forjar completion --shell bash > /etc/bash_completion.d/forjar 2>/dev/null || true
    forjar completion --shell zsh > "${fpath[1]}/_forjar" 2>/dev/null || true
```

**Release workflow integration**:

```yaml
# In .github/workflows/release.yml, after building binaries:
- name: Generate distribution artifacts
  run: |
    forjar dist -f forjar.yaml --all --output-dir dist/
    forjar dist -f forjar.yaml --verify

- name: Update Homebrew tap
  run: |
    cp dist/forjar.rb ../homebrew-tap/Formula/forjar.rb
    cd ../homebrew-tap && git add . && git commit -m "forjar $VERSION" && git push

- name: Upload install script
  run: |
    aws s3 cp dist/install.sh s3://forjar.dev/install.sh --content-type text/plain
```

---

## Falsification Criteria

Per Popper (1959), the following must hold or the feature is measuring the wrong thing:

| ID | Assertion | Test |
|----|-----------|------|
| F-3601 | Shell installer works on Alpine (musl, no bash) | Run `sh install.sh` in `alpine:latest` container |
| F-3602 | Shell installer detects ARM correctly | Run in `arm64v8/ubuntu` container, verify aarch64 asset downloaded |
| F-3603 | Shell installer fails gracefully with no curl/wget | Remove both, verify error message names the missing tool |
| F-3604 | Shell installer verifies checksum before install | Corrupt downloaded archive, verify install aborts |
| F-3605 | Homebrew formula installs on macOS | `brew install --build-from-source` on macOS runner |
| F-3606 | Nix flake builds on x86_64-linux | `nix build` in NixOS container |
| F-3607 | cargo-binstall metadata resolves correctly | `cargo binstall forjar --dry-run` succeeds |
| F-3608 | Generated artifacts use real checksums, not placeholders | grep for "TODO\|PLACEHOLDER\|000000" in output — must find zero |
| F-3609 | `--verify` catches a broken installer | Deliberately break asset URL, verify `--verify` reports failure |
| F-3610 | Version pinning produces reproducible output | `forjar dist --installer --version v1.1.1` run twice produces identical output |

---

## Implementation Plan

### Phase A: Core Types and Shell Installer (FJ-3601)

1. Add `DistConfig` type to `src/core/types/`
2. Parse `dist:` section in config
3. Implement `forjar dist --installer` — the highest-value artifact
4. Add `forjar dist --verify` for installer only (Alpine + Ubuntu containers)
5. Dogfood: generate forjar's own `install.sh`, host at `forjar.dev/install.sh`

### Phase B: Package Manager Formats (FJ-3602–3604)

6. Implement `--homebrew` (Ruby formula generation)
7. Implement `--binstall` (TOML fragment)
8. Implement `--nix` (flake.nix generation)
9. Checksum resolution from GitHub Release assets

### Phase C: CI and OS Packages (FJ-3605–3606)

10. Implement `--github-action` (action.yml)
11. Implement `--deb` and `--rpm` spec generation
12. Full `--all` and `--output-dir` support

### Phase D: Verification (FJ-3607)

13. Container-based verification for each artifact type
14. Integration with release workflow
15. `--verify` runs all applicable checks

---

## Relationship to Existing Features

| Existing Feature | How `dist` Builds On It |
|-----------------|------------------------|
| `github_release` resource type | Same asset resolution pattern (repo, tag, asset_pattern) |
| Shell codegen (`src/core/codegen/`) | Same script generation infrastructure |
| BLAKE3 integrity | Checksum verification in generated scripts |
| `forjar completion` | Post-install hook generates completions |
| `forjar bundle` | Both package project artifacts; bundle = config+state, dist = binary+installer |
| `forjar image` | Both generate installable artifacts; image = OS image, dist = package |
| Template expansion (`{{version}}`) | Asset URLs use same template syntax |
| `forjar schema` | JSON Schema includes `dist:` section |
| `forjar build --push` | OCI push pattern informs registry upload |

---

## Non-Goals

- **Building binaries** — `forjar dist` generates distribution artifacts for pre-built binaries. Cross-compilation is handled by `forjar build` or CI.
- **Package hosting** — forjar generates the formula/flake/spec but does not host a package repository. Users push to their own Homebrew tap, Nix cache, etc.
- **Windows MSI/exe** — out of scope for v1. Windows users use WSL or `cargo install`.
- **Auto-publishing** — `forjar dist` generates artifacts. Publishing to registries is a CI step, not a forjar responsibility.
