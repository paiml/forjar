//! FJ-54: Tests for Android Magisk module generation.

#[cfg(test)]
mod tests {
    use crate::cli::image_android::*;
    use std::path::Path;

    fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
        let p = dir.join("forjar.yaml");
        std::fs::write(&p, yaml).unwrap();
        p
    }

    const ANDROID_MACHINE_YAML: &str = r#"
version: "1.0"
name: pixel-fleet
machines:
  pixel:
    hostname: pixel-compute
    addr: 192.168.50.100
    arch: aarch64
    user: root
resources:
  pkg-curl:
    type: package
    machine: pixel
    provider: apt
    packages: [curl]
"#;

    #[test]
    fn test_fj54_magisk_module_generates() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), ANDROID_MACHINE_YAML);
        let output = dir.path().join("forjar-magisk.zip");
        let result = cmd_image_android(&p, Some("pixel"), &output, false);
        assert!(result.is_ok(), "magisk module must generate: {result:?}");
        assert!(output.exists(), "zip file must exist");
        assert!(output.metadata().unwrap().len() > 0, "zip must not be empty");
    }

    #[test]
    fn test_fj54_magisk_module_json() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), ANDROID_MACHINE_YAML);
        let output = dir.path().join("forjar-magisk.zip");
        let result = cmd_image_android(&p, Some("pixel"), &output, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj54_magisk_zip_structure() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), ANDROID_MACHINE_YAML);
        let output = dir.path().join("forjar-magisk.zip");
        cmd_image_android(&p, Some("pixel"), &output, false).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();

        assert!(names.contains(&"module.prop".to_string()), "must have module.prop: {names:?}");
        assert!(names.contains(&"service.sh".to_string()), "must have service.sh: {names:?}");
        assert!(
            names.contains(&"post-fs-data.sh".to_string()),
            "must have post-fs-data.sh: {names:?}"
        );
        assert!(
            names.contains(&"customize.sh".to_string()),
            "must have customize.sh: {names:?}"
        );
        assert!(
            names.contains(&"system/etc/init/forjar.rc".to_string()),
            "must have init.rc: {names:?}"
        );
        assert!(
            names.contains(&"system/etc/forjar/forjar.yaml".to_string()),
            "must have config: {names:?}"
        );
        assert!(
            names.contains(&"system/bin/forjar".to_string()),
            "must have binary stub: {names:?}"
        );
    }

    #[test]
    fn test_fj54_module_prop_content() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), ANDROID_MACHINE_YAML);
        let output = dir.path().join("forjar-magisk.zip");
        cmd_image_android(&p, Some("pixel"), &output, false).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut prop = archive.by_name("module.prop").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut prop, &mut content).unwrap();

        assert!(content.contains("id=forjar-pixel"), "module id: {content}");
        assert!(content.contains("minMagisk=25000"), "min magisk version: {content}");
    }

    #[test]
    fn test_fj54_init_rc_content() {
        let rc = generate_init_rc("pixel-compute");
        assert!(rc.contains("service forjar"), "must define forjar service");
        assert!(rc.contains("/system/bin/forjar"), "must reference forjar binary");
        assert!(
            rc.contains("/system/etc/forjar/forjar.yaml"),
            "must reference config"
        );
        assert!(rc.contains("oneshot"), "must be oneshot service");
        assert!(
            rc.contains("sys.boot_completed=1"),
            "must trigger on boot_completed"
        );
    }

    #[test]
    fn test_fj54_service_sh_content() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), ANDROID_MACHINE_YAML);
        let output = dir.path().join("forjar-magisk.zip");
        cmd_image_android(&p, Some("pixel"), &output, false).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut svc = archive.by_name("service.sh").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut svc, &mut content).unwrap();

        assert!(
            content.contains("apply --yes"),
            "service.sh must run forjar apply"
        );
        assert!(
            content.contains(".firstboot-done"),
            "service.sh must be idempotent"
        );
    }

    #[test]
    fn test_fj54_customize_sh_content() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), ANDROID_MACHINE_YAML);
        let output = dir.path().join("forjar-magisk.zip");
        cmd_image_android(&p, Some("pixel"), &output, false).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut cs = archive.by_name("customize.sh").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut cs, &mut content).unwrap();

        assert!(content.contains("set_perm"), "must set permissions");
        assert!(content.contains("0755"), "binary must be executable");
    }

    #[test]
    fn test_fj54_config_embedded() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), ANDROID_MACHINE_YAML);
        let output = dir.path().join("forjar-magisk.zip");
        cmd_image_android(&p, Some("pixel"), &output, false).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut cfg = archive.by_name("system/etc/forjar/forjar.yaml").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut cfg, &mut content).unwrap();

        assert!(
            content.contains("pixel-fleet"),
            "config must contain project name"
        );
        assert!(
            content.contains("pixel-compute"),
            "config must contain machine hostname"
        );
    }

    #[test]
    fn test_fj54_binary_stub() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), ANDROID_MACHINE_YAML);
        let output = dir.path().join("forjar-magisk.zip");
        cmd_image_android(&p, Some("pixel"), &output, false).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut bin = archive.by_name("system/bin/forjar").unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut bin, &mut content).unwrap();

        assert!(
            content.contains("cross-compile"),
            "stub must mention cross-compilation"
        );
        assert!(content.contains("aarch64"), "stub must mention target arch");
    }

    #[test]
    fn test_fj54_missing_machine() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), ANDROID_MACHINE_YAML);
        let output = dir.path().join("forjar-magisk.zip");
        let result = cmd_image_android(&p, Some("nonexistent"), &output, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_fj54_auto_resolve_single_machine() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), ANDROID_MACHINE_YAML);
        let output = dir.path().join("forjar-magisk.zip");
        let result = cmd_image_android(&p, None, &output, false);
        assert!(result.is_ok(), "single machine auto-resolve: {result:?}");
    }
}
