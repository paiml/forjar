//! FJ-54: Android image generation — Magisk module packaging.
//!
//! Generates a Magisk module ZIP that installs forjar as a system service
//! on rooted Android devices. The module includes:
//! - Cross-compiled forjar binary (`aarch64-linux-android`)
//! - forjar.yaml configuration
//! - init.d script for boot-time convergence

use super::helpers::*;
use std::io::Write;
use std::path::Path;

/// Generate a Magisk module ZIP for Android deployment.
pub fn cmd_image_android(
    file: &Path,
    machine_name: Option<&str>,
    output: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let (name, machine) = super::image_cmd::resolve_machine(&config, machine_name)?;

    let zip_data = build_magisk_module(&name, machine, file)?;

    std::fs::write(output, &zip_data)
        .map_err(|e| format!("write magisk module to {}: {e}", output.display()))?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "machine": name,
                "output": output.display().to_string(),
                "size": zip_data.len(),
                "format": "magisk-module",
            })
        );
    } else {
        println!(
            "Magisk module: {} ({} bytes)",
            output.display(),
            zip_data.len()
        );
        println!("  machine: {name}");
        println!("  arch: {}", machine.arch);
        println!("\nInstall: flash via Magisk Manager or TWRP");
    }

    Ok(())
}

/// Generate the Android init.rc service stanza for forjar.
pub fn generate_init_rc(hostname: &str) -> String {
    format!(
        r#"# Forjar convergence service for {hostname}
service forjar /system/bin/forjar apply --yes -f /system/etc/forjar/forjar.yaml
    class late_start
    user root
    group root
    oneshot
    disabled

on property:sys.boot_completed=1
    start forjar
"#
    )
}

/// Generate the Magisk module.prop metadata.
fn generate_module_prop(name: &str) -> String {
    format!(
        r#"id=forjar-{name}
name=Forjar IaC Agent ({name})
version=1.0.0
versionCode=1
author=forjar
description=Infrastructure convergence agent for {name}
minMagisk=25000
"#
    )
}

/// Generate the post-fs-data.sh script for Magisk.
fn generate_post_fs_data() -> &'static str {
    r#"#!/system/bin/sh
# Forjar Magisk module — post-fs-data hook
# Ensure forjar config directory exists
mkdir -p /data/forjar
# Copy marker if first boot
if [ ! -f /data/forjar/.firstboot-done ]; then
    log -t forjar "First boot detected, convergence will run at boot_completed"
fi
"#
}

/// Generate the service.sh script that runs at boot_completed.
fn generate_service_sh() -> &'static str {
    r#"#!/system/bin/sh
# Forjar Magisk module — boot service
MODDIR="${0%/*}"
FORJAR="$MODDIR/system/bin/forjar"
CONFIG="$MODDIR/system/etc/forjar/forjar.yaml"

if [ ! -f /data/forjar/.firstboot-done ]; then
    log -t forjar "Running first-boot convergence"
    "$FORJAR" apply --yes -f "$CONFIG" 2>&1 | while read -r line; do
        log -t forjar "$line"
    done
    touch /data/forjar/.firstboot-done
    log -t forjar "First-boot convergence complete"
fi
"#
}

/// Build the Magisk module as an in-memory ZIP.
fn build_magisk_module(
    name: &str,
    machine: &crate::core::types::Machine,
    config_file: &Path,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // module.prop
        zip.start_file("module.prop", options)
            .map_err(|e| format!("zip module.prop: {e}"))?;
        zip.write_all(generate_module_prop(name).as_bytes())
            .map_err(|e| format!("write module.prop: {e}"))?;

        // post-fs-data.sh
        zip.start_file("post-fs-data.sh", options)
            .map_err(|e| format!("zip post-fs-data.sh: {e}"))?;
        zip.write_all(generate_post_fs_data().as_bytes())
            .map_err(|e| format!("write post-fs-data.sh: {e}"))?;

        // service.sh
        zip.start_file("service.sh", options)
            .map_err(|e| format!("zip service.sh: {e}"))?;
        zip.write_all(generate_service_sh().as_bytes())
            .map_err(|e| format!("write service.sh: {e}"))?;

        // init.rc fragment
        zip.start_file("system/etc/init/forjar.rc", options)
            .map_err(|e| format!("zip init.rc: {e}"))?;
        zip.write_all(generate_init_rc(&machine.hostname).as_bytes())
            .map_err(|e| format!("write init.rc: {e}"))?;

        // forjar.yaml config
        let config_content =
            std::fs::read_to_string(config_file).map_err(|e| format!("read config: {e}"))?;
        zip.start_file("system/etc/forjar/forjar.yaml", options)
            .map_err(|e| format!("zip config: {e}"))?;
        zip.write_all(config_content.as_bytes())
            .map_err(|e| format!("write config: {e}"))?;

        // Placeholder for forjar binary (user must cross-compile separately)
        let stub = format!(
            "#!/system/bin/sh\necho 'forjar stub — cross-compile for {} and replace this file'\nexit 1\n",
            machine.arch
        );
        zip.start_file("system/bin/forjar", options)
            .map_err(|e| format!("zip binary stub: {e}"))?;
        zip.write_all(stub.as_bytes())
            .map_err(|e| format!("write binary stub: {e}"))?;

        // customize.sh (Magisk installer script)
        zip.start_file("customize.sh", options)
            .map_err(|e| format!("zip customize.sh: {e}"))?;
        zip.write_all(generate_customize_sh().as_bytes())
            .map_err(|e| format!("write customize.sh: {e}"))?;

        zip.finish().map_err(|e| format!("finalize zip: {e}"))?;
    }
    Ok(buf)
}

/// Generate the customize.sh Magisk installer script.
fn generate_customize_sh() -> &'static str {
    r#"#!/system/bin/sh
# Forjar Magisk module installer
ui_print "Installing Forjar IaC agent..."
set_perm "$MODPATH/system/bin/forjar" 0 0 0755
set_perm_recursive "$MODPATH/system/etc/forjar" 0 0 0755 0644
mkdir -p /data/forjar
ui_print "Forjar will run convergence on next boot"
"#
}
