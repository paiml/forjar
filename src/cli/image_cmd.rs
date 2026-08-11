//! FJ-52: Autoinstall ISO generation from forjar.yaml.
//!
//! Generates Ubuntu autoinstall `user-data` YAML from the machines: section,
//! and optionally repacks a base ISO with xorriso to produce a bootable image.

use super::helpers::*;
use std::path::Path;

/// Generate autoinstall user-data YAML for a machine.
#[allow(clippy::too_many_arguments)]
pub fn cmd_image_user_data(
    file: &Path,
    machine_name: Option<&str>,
    disk: &str,
    locale: &str,
    timezone: &str,
    output: Option<&Path>,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let (name, machine) = resolve_machine(&config, machine_name)?;
    let user_data = generate_user_data(&name, machine, disk, locale, timezone)?;

    if let Some(out) = output {
        std::fs::write(out, &user_data)
            .map_err(|e| format!("write user-data to {}: {e}", out.display()))?;
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "machine": name,
                    "output": out.display().to_string(),
                    "size": user_data.len(),
                })
            );
        } else {
            println!("Wrote user-data for '{name}' to {}", out.display());
        }
    } else {
        print!("{user_data}");
    }
    Ok(())
}

/// Generate a bootable autoinstall ISO by repacking a base Ubuntu ISO.
#[allow(clippy::too_many_arguments)]
pub fn cmd_image_iso(
    file: &Path,
    machine_name: Option<&str>,
    base_iso: &Path,
    output: &Path,
    disk: &str,
    locale: &str,
    timezone: &str,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let (name, machine) = resolve_machine(&config, machine_name)?;

    if !base_iso.exists() {
        return Err(format!("base ISO not found: {}", base_iso.display()));
    }
    ensure_xorriso()?;

    let user_data = generate_user_data(&name, machine, disk, locale, timezone)?;

    // Create temp working directory
    let work_id = std::process::id();
    let work = std::env::temp_dir().join(format!("forjar-image-{work_id}"));
    std::fs::create_dir_all(&work).map_err(|e| format!("create work dir: {e}"))?;

    // Extract base ISO and stage the autoinstall payload
    stage_iso_workdir(&work, base_iso, &user_data, file)?;

    // Repack ISO with xorriso
    repack_iso(&work, output)?;

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&work);

    print_iso_result(output, &name, base_iso, json);
    Ok(())
}

/// Verify xorriso is installed and on PATH.
fn ensure_xorriso() -> Result<(), String> {
    let has_xorriso = std::process::Command::new("which")
        .arg("xorriso")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());

    if has_xorriso {
        Ok(())
    } else {
        Err("xorriso not found — install with: sudo apt install xorriso".to_string())
    }
}

/// Extract the base ISO and stage the autoinstall payload
/// (user-data, meta-data, forjar binary, forjar.yaml) into the work dir.
fn stage_iso_workdir(
    work: &Path,
    base_iso: &Path,
    user_data: &str,
    file: &Path,
) -> Result<(), String> {
    // Extract base ISO
    extract_iso(base_iso, work)?;

    // Write user-data and meta-data
    let nocloud = work.join("nocloud");
    std::fs::create_dir_all(&nocloud).map_err(|e| format!("create nocloud dir: {e}"))?;
    std::fs::write(nocloud.join("user-data"), user_data)
        .map_err(|e| format!("write user-data: {e}"))?;
    std::fs::write(nocloud.join("meta-data"), "").map_err(|e| format!("write meta-data: {e}"))?;

    // Copy forjar binary into ISO
    embed_forjar_binary(work)?;

    // Copy forjar.yaml into ISO
    let iso_config_dir = work.join("forjar");
    std::fs::create_dir_all(&iso_config_dir).map_err(|e| format!("create forjar dir: {e}"))?;
    std::fs::copy(file, iso_config_dir.join("forjar.yaml"))
        .map_err(|e| format!("copy config: {e}"))?;
    Ok(())
}

/// Print the ISO generation summary (JSON or human-readable).
fn print_iso_result(output: &Path, name: &str, base_iso: &Path, json: bool) {
    let size = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "machine": name,
                "base": base_iso.display().to_string(),
                "output": output.display().to_string(),
                "size": size,
            })
        );
    } else {
        let size_mb = size / (1024 * 1024);
        println!("ISO generated: {} ({size_mb} MB)", output.display());
        println!("  machine: {name}");
        println!("  base: {}", base_iso.display());
        println!(
            "\nWrite to USB: dd if={} of=/dev/sdX bs=4M status=progress",
            output.display()
        );
    }
}

/// Resolve the target machine from config.
pub fn resolve_machine<'a>(
    config: &'a crate::core::types::ForjarConfig,
    machine_name: Option<&str>,
) -> Result<(String, &'a crate::core::types::Machine), String> {
    if let Some(name) = machine_name {
        let m = config
            .machines
            .get(name)
            .ok_or_else(|| format!("machine '{name}' not found in config"))?;
        return Ok((name.to_string(), m));
    }
    let mut iter = config.machines.iter();
    match (iter.next(), iter.next()) {
        (Some((name, m)), None) => Ok((name.clone(), m)),
        (None, _) => Err("no machines defined in config".to_string()),
        (Some(_), Some(_)) => {
            let names: Vec<_> = config.machines.keys().collect();
            Err(format!(
                "multiple machines found, specify one with --machine: {names:?}"
            ))
        }
    }
}

/// Generate Ubuntu autoinstall user-data YAML.
pub fn generate_user_data(
    name: &str,
    machine: &crate::core::types::Machine,
    disk: &str,
    locale: &str,
    timezone: &str,
) -> Result<String, String> {
    // GH-91: name not yet used for user-data personalization
    let _ = name;
    let hostname = &machine.hostname;
    let username = &machine.user;

    // Read SSH public key if specified
    let ssh_keys = read_ssh_pub_key(machine.ssh_key.as_deref())?;

    // Determine storage layout
    let storage_layout = storage_layout_yaml(disk)?;

    let mut yaml = String::from("#cloud-config\nautoinstall:\n  version: 1\n");
    yaml.push_str(&format!("  locale: {locale}\n"));
    yaml.push_str("  keyboard:\n    layout: us\n");
    yaml.push_str(&format!("  timezone: {timezone}\n"));

    // Identity
    yaml.push_str("  identity:\n");
    yaml.push_str(&format!("    hostname: {hostname}\n"));
    yaml.push_str(&format!("    username: {username}\n"));

    // SSH
    yaml.push_str("  ssh:\n    install-server: true\n");
    if !ssh_keys.is_empty() {
        yaml.push_str("    authorized-keys:\n");
        for key in &ssh_keys {
            yaml.push_str(&format!("      - {key}\n"));
        }
    }

    // Storage
    yaml.push_str("  storage:\n");
    yaml.push_str(&storage_layout);
    yaml.push('\n');

    // Packages
    yaml.push_str("  packages:\n    - openssh-server\n    - curl\n");

    // Late commands: sudo, forjar binary, config, firstboot service
    yaml.push_str("  late-commands:\n");

    // Passwordless sudo
    yaml.push_str(&format!(
        "    - curtin in-target -- bash -c 'echo \"{username} ALL=(ALL) NOPASSWD:ALL\" > /etc/sudoers.d/{username}-nopasswd'\n"
    ));
    yaml.push_str(&format!(
        "    - curtin in-target -- chmod 0440 /etc/sudoers.d/{username}-nopasswd\n"
    ));

    // Copy forjar binary and config
    yaml.push_str(
        "    - cp /cdrom/forjar/bin/forjar /target/usr/local/bin/forjar 2>/dev/null || true\n",
    );
    yaml.push_str("    - mkdir -p /target/etc/forjar\n");
    yaml.push_str(
        "    - cp /cdrom/forjar/forjar.yaml /target/etc/forjar/forjar.yaml 2>/dev/null || true\n",
    );

    // Firstboot systemd service
    yaml.push_str(&firstboot_service_command());

    // Network config from machine addr (static IP if specified)
    if machine.addr != "127.0.0.1" && machine.addr != "container" && machine.addr != "pepita" {
        yaml.push_str(&format!(
            "  # Static IP: {}\n  # Configure via netplan in late-commands if needed\n",
            machine.addr
        ));
    }

    Ok(yaml)
}

/// Map the disk argument to an autoinstall storage layout YAML snippet.
fn storage_layout_yaml(disk: &str) -> Result<String, String> {
    match disk {
        "auto-lvm" => Ok("    layout:\n      name: lvm".to_string()),
        "auto-zfs" => Ok("    layout:\n      name: zfs".to_string()),
        path if path.starts_with('/') => Ok(format!(
            "    layout:\n      name: lvm\n    config:\n      - type: disk\n        id: disk0\n        path: {path}"
        )),
        other => Err(format!(
            "unknown disk layout: {other} (use auto-lvm, auto-zfs, or /dev/path)"
        )),
    }
}

/// Read SSH public key file (tries .pub extension).
pub fn read_ssh_pub_key(ssh_key: Option<&str>) -> Result<Vec<String>, String> {
    let Some(key_path) = ssh_key else {
        return Ok(Vec::new());
    };

    // Try the .pub version first
    let pub_path = if key_path.ends_with(".pub") {
        key_path.to_string()
    } else {
        format!("{key_path}.pub")
    };

    let expanded = expand_tilde(&pub_path);
    match std::fs::read_to_string(&expanded) {
        Ok(content) => Ok(content
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect()),
        Err(_) => Ok(Vec::new()),
    }
}

/// Expand ~ to $HOME in a path string.
pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

/// Re-indent every line of an embedded multi-line block by `n` spaces.
///
/// Refs #212 (`machine-output-parses-v1`): a `|` block scalar in YAML claims
/// every line of its body, and a line that starts at column 0 ENDS the scalar
/// and is re-parsed as a mapping key. Interpolating a heredoc body without
/// re-indenting it therefore produced a `user-data` that no cloud-init (and no
/// YAML parser) can read — while `image --user-data` exited 0 saying "Wrote
/// user-data". Blank lines stay blank: trailing whitespace is not content, and
/// YAML treats an empty line inside a block scalar as a newline either way.
///
/// The block indentation is stripped again by the YAML loader, so the shell
/// that eventually runs the body sees it back at column 0 — which is what a
/// `<<'UNIT'` heredoc terminator requires.
pub fn indent_block(text: &str, n: usize) -> String {
    let pad = " ".repeat(n);
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{pad}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate the late-command that installs the forjar-firstboot systemd service.
pub fn firstboot_service_command() -> String {
    let unit = r#"[Unit]
Description=Forjar First Boot Convergence
After=network-online.target
Wants=network-online.target
ConditionPathExists=!/etc/forjar/.firstboot-done

[Service]
Type=oneshot
ExecStart=/usr/local/bin/forjar apply --yes -f /etc/forjar/forjar.yaml
ExecStartPost=/usr/bin/touch /etc/forjar/.firstboot-done
TimeoutSec=1800

[Install]
WantedBy=multi-user.target"#;

    // Refs #212: the body is indented INTO the block scalar (6 spaces, matching
    // the `cat >` line), including the `UNIT` terminator. YAML strips those 6
    // spaces on load, so the shell gets the unit at column 0 and the heredoc
    // terminates. The previous code emitted the body at column 0 in the YAML,
    // which broke the document AND left the heredoc unterminated in the shell.
    const BLOCK_INDENT: usize = 6;
    let mut cmd = String::new();
    cmd.push_str("    - |\n");
    cmd.push_str("      cat > /target/etc/systemd/system/forjar-firstboot.service <<'UNIT'\n");
    cmd.push_str(&indent_block(unit, BLOCK_INDENT));
    cmd.push('\n');
    cmd.push_str(&indent_block("UNIT", BLOCK_INDENT));
    cmd.push('\n');
    cmd.push_str("    - curtin in-target -- systemctl enable forjar-firstboot\n");
    cmd
}

/// Extract a base ISO using xorriso.
fn extract_iso(base_iso: &Path, work_dir: &Path) -> Result<(), String> {
    let extract_dir = work_dir.join("iso");
    std::fs::create_dir_all(&extract_dir).map_err(|e| format!("create iso dir: {e}"))?;

    let status = std::process::Command::new("xorriso")
        .args([
            "-osirrox",
            "on",
            "-indev",
            &base_iso.display().to_string(),
            "-extract",
            "/",
            &extract_dir.display().to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .map_err(|e| format!("run xorriso extract: {e}"))?;

    if !status.success() {
        return Err("xorriso extraction failed".to_string());
    }
    Ok(())
}

/// Copy the current forjar binary into the ISO working directory.
fn embed_forjar_binary(work_dir: &Path) -> Result<(), String> {
    let bin_dir = work_dir.join("iso").join("forjar").join("bin");
    std::fs::create_dir_all(&bin_dir).map_err(|e| format!("create bin dir: {e}"))?;
    if let Ok(exe) = std::env::current_exe() {
        std::fs::copy(&exe, bin_dir.join("forjar"))
            .map_err(|e| format!("copy forjar binary: {e}"))?;
    }
    Ok(())
}

/// Repack an extracted ISO directory into a bootable ISO using xorriso.
fn repack_iso(work_dir: &Path, output: &Path) -> Result<(), String> {
    let iso_dir = work_dir.join("iso");

    let status = std::process::Command::new("xorriso")
        .args([
            "-as",
            "mkisofs",
            "-r",
            "-V",
            "FORJAR_AUTOINSTALL",
            "-o",
            &output.display().to_string(),
            "-J",
            "-joliet-long",
            "-b",
            "boot/grub/i386-pc/eltorito.img",
            "-c",
            "boot.catalog",
            "-no-emul-boot",
            "-boot-load-size",
            "4",
            "-boot-info-table",
            "--grub2-boot-info",
            "--grub2-mbr",
            &iso_dir
                .join("boot/grub/i386-pc/boot_hybrid.img")
                .display()
                .to_string(),
            "-eltorito-alt-boot",
            "-e",
            "boot/grub/efi.img",
            "-no-emul-boot",
            &iso_dir.display().to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .map_err(|e| format!("run xorriso repack: {e}"))?;

    if !status.success() {
        return Err("xorriso ISO repack failed".to_string());
    }
    Ok(())
}
