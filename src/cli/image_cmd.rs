//! FJ-52: Autoinstall ISO generation from forjar.yaml.
//!
//! Generates Ubuntu autoinstall `user-data` YAML from the machines: section,
//! and optionally repacks a base ISO with xorriso to produce a bootable image.

use super::helpers::*;
use super::image_iso_pack::{embed_forjar_binary, extract_iso_file, repack_iso};
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

    // Stage the autoinstall payload and the patched grub configs
    let grub_cfgs = stage_iso_workdir(&work, base_iso, &user_data, file)?;

    // Repack: the base ISO's boot records replayed, the payload mapped in
    repack_iso(&work, base_iso, output, &grub_cfgs)?;

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

/// Grub configs that boot the installer; each `linux` line gets the
/// autoinstall cmdline.
const GRUB_CFGS: [&str; 2] = ["/boot/grub/grub.cfg", "/boot/grub/loopback.cfg"];

/// Stage the autoinstall payload (user-data, meta-data, forjar binary,
/// forjar.yaml) and patched grub configs under the work dir. Returns the
/// grub configs found in the base ISO, as (ISO path, staged file).
///
/// #731: this used to extract the whole ISO to `work/iso` and write the
/// payload to `work/nocloud` and `work/forjar`, OUTSIDE the tree `repack_iso`
/// packed, so the stick never carried its seed.
fn stage_iso_workdir(
    work: &Path,
    base_iso: &Path,
    user_data: &str,
    file: &Path,
) -> Result<Vec<(String, std::path::PathBuf)>, String> {
    let nocloud = work.join("nocloud");
    std::fs::create_dir_all(&nocloud).map_err(|e| format!("create nocloud dir: {e}"))?;
    std::fs::write(nocloud.join("user-data"), user_data)
        .map_err(|e| format!("write user-data: {e}"))?;
    std::fs::write(nocloud.join("meta-data"), "").map_err(|e| format!("write meta-data: {e}"))?;

    embed_forjar_binary(work).map_err(crate::core::error::ForjarError::into_untyped)?;
    std::fs::copy(file, work.join("forjar").join("forjar.yaml"))
        .map_err(|e| format!("copy config: {e}"))?;

    let grub_dir = work.join("grub");
    std::fs::create_dir_all(&grub_dir).map_err(|e| format!("create grub dir: {e}"))?;
    let mut staged = Vec::new();
    for iso_path in GRUB_CFGS {
        let dst = grub_dir.join(iso_path.rsplit('/').next().unwrap_or("grub.cfg"));
        if !extract_iso_file(base_iso, iso_path, &dst)? {
            continue;
        }
        let cfg = std::fs::read_to_string(&dst).map_err(|e| format!("read {iso_path}: {e}"))?;
        // xorriso -extract keeps the ISO's 0444: replace the file, don't write into it.
        std::fs::remove_file(&dst).map_err(|e| format!("replace {iso_path}: {e}"))?;
        std::fs::write(&dst, add_autoinstall_cmdline(&cfg))
            .map_err(|e| format!("write {iso_path}: {e}"))?;
        staged.push((iso_path.to_string(), dst));
    }
    if staged.is_empty() {
        return Err(format!(
            "{}: no /boot/grub/grub.cfg — not an Ubuntu live ISO",
            base_iso.display()
        ));
    }
    Ok(staged)
}

/// Add `autoinstall ds=nocloud;s=/cdrom/nocloud/` to every `linux` line, before
/// the `---` separator when there is one. grub reads `;` as a command separator,
/// so the ds= value is single-quoted. Lines that already say `autoinstall` are
/// left alone.
pub fn add_autoinstall_cmdline(cfg: &str) -> String {
    const ARGS: &str = " autoinstall 'ds=nocloud;s=/cdrom/nocloud/'";
    cfg.lines()
        .map(|line| {
            let t = line.trim_start();
            if !t.starts_with("linux") || t.starts_with("linux16") || line.contains("autoinstall") {
                return line.to_string();
            }
            match line.find(" ---") {
                Some(i) => format!("{}{ARGS}{}", &line[..i], &line[i..]),
                None => format!("{line}{ARGS}"),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if cfg.ends_with('\n') { "\n" } else { "" }
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
    let hostname = &machine.hostname;
    let username = &machine.user;

    // #731: the only way into the installed box is a key (no password login),
    // so an image with no key is an image of an unreachable machine.
    let ssh_keys = read_ssh_pub_key(machine.ssh_key.as_deref())?;
    if ssh_keys.is_empty() {
        return Err(format!(
            "machine '{name}' has no ssh_key: the installed box would be unreachable \
             (password login is disabled); set machines.{name}.ssh_key"
        ));
    }

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
    // #731: subiquity's schema requires identity.password; without it the
    // install stops at an interactive prompt. '!' is a locked password: keys only.
    yaml.push_str("    password: '!'\n");

    // SSH
    yaml.push_str("  ssh:\n    install-server: true\n    allow-pw: false\n");
    yaml.push_str("    authorized-keys:\n");
    for key in &ssh_keys {
        yaml.push_str(&format!("      - {}\n", yaml_single_quoted(key)));
    }

    // #731: DHCP on any wired NIC, during the install and on the installed box
    // (this used to be a comment saying "configure via netplan if needed").
    yaml.push_str(
        "  network:\n    version: 2\n    ethernets:\n      wired:\n        match:\n          name: 'en*'\n        dhcp4: true\n",
    );

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

    Ok(yaml)
}

/// Map the disk argument to an autoinstall storage layout YAML snippet.
///
/// #731: the pin MUST be `layout.match`. subiquity ignores `storage.config`
/// whenever `storage.layout` is present, so the old `layout` + `config: [disk
/// path]` pair installed onto the largest disk it found while looking pinned.
/// A `serial:` glob is the safe pin: a stick that lands in the wrong machine
/// matches no disk and fails instead of wiping one.
///
/// LVM layouts take `sizing-policy: all`. subiquity's default, `scaled`, makes
/// a ~100 GB root and leaves the rest of the disk unallocated: on a build host
/// that is a full disk at a fifth of its size.
fn storage_layout_yaml(disk: &str) -> Result<String, String> {
    match disk {
        "auto-lvm" => Ok("    layout:\n      name: lvm\n      sizing-policy: all".to_string()),
        "auto-zfs" => Ok("    layout:\n      name: zfs".to_string()),
        serial if serial.starts_with("serial:") => {
            let glob = &serial["serial:".len()..];
            if glob.is_empty() {
                return Err("--disk serial: needs a glob, e.g. serial:*S58SNMFX200010W*".to_string());
            }
            // udev's NVMe ID_SERIAL ends in the namespace id (measured:
            // Samsung_SSD_970_EVO_Plus_500GB_<serial>_1), so a glob anchored at the
            // end matches no NVMe disk and subiquity stops mid-install with "Failed
            // to find matching device". A trailing * also matches SATA serials.
            if !glob.ends_with('*') {
                return Err(format!(
                    "--disk serial:{glob} matches no NVMe disk (udev's ID_SERIAL ends in \
                     _<namespace>, e.g. ..._{tail}_1); use serial:{glob}*",
                    tail = glob.trim_start_matches('*')
                ));
            }
            Ok(format!(
                "    layout:\n      name: lvm\n      sizing-policy: all\n      match:\n        serial: {}",
                yaml_single_quoted(glob)
            ))
        }
        link if link.starts_with("/dev/disk/") => Err(format!(
            "{link}: subiquity matches a disk by kernel path (/dev/nvme0n1) or serial, never \
             by a /dev/disk/by-* link, so this would match nothing; use --disk serial:<glob> \
             (lsblk -dno SERIAL <dev>)"
        )),
        path if path.starts_with('/') => Ok(format!(
            "    layout:\n      name: lvm\n      sizing-policy: all\n      match:\n        path: {}",
            yaml_single_quoted(path)
        )),
        other => Err(format!(
            "unknown disk layout: {other} (use auto-lvm, auto-zfs, serial:<glob> or /dev/<kernel name>)"
        )),
    }
}

/// YAML single-quoted scalar ('' is the only escape). A serial glob starts
/// with '*', which unquoted YAML reads as an alias.
fn yaml_single_quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Read the public half of a machine's `ssh_key`.
///
/// #731: an unreadable key file used to return `Ok(vec![])`, and the image
/// built "successfully" for a box no one could log into.
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
    let content = std::fs::read_to_string(&expanded)
        .map_err(|e| format!("ssh_key public half {expanded}: {e}"))?;
    Ok(content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect())
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
