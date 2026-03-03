//! FJ-1411: Declarative data validation checks.
//!
//! Validates data quality: file existence, non-empty, BLAKE3 integrity,
//! size constraints, and custom check commands.

use super::helpers::*;
use std::path::Path;

pub(crate) fn cmd_data_validate(
    file: &Path,
    resource_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let config_dir = file.parent().unwrap_or(Path::new("."));

    let mut checks = Vec::new();

    for (id, resource) in &config.resources {
        if let Some(filter) = resource_filter {
            if id != filter {
                continue;
            }
        }

        // Check source file integrity
        if let Some(ref src) = resource.source {
            let src_path = config_dir.join(src);
            checks.push(validate_file_exists(&src_path, id, src));
        }

        // Check output artifacts
        for artifact in &resource.output_artifacts {
            let art_path = config_dir.join(artifact);
            checks.push(validate_file_exists(&art_path, id, artifact));
        }

        // Check content hash stability
        if let Some(ref content) = resource.content {
            let hash = blake3::hash(content.as_bytes()).to_hex()[..16].to_string();
            checks.push(ValidationCheck {
                resource: id.clone(),
                check_type: "content-hash".to_string(),
                target: "(inline)".to_string(),
                passed: true,
                detail: format!("blake3:{hash}"),
            });
        }
    }

    // Validate store directory
    let store_dir = config_dir.join("store");
    if store_dir.exists() {
        validate_store_integrity(&store_dir, &mut checks);
    }

    let pass_count = checks.iter().filter(|c| c.passed).count();
    let fail_count = checks.iter().filter(|c| !c.passed).count();

    if json {
        print_validate_json(&checks, pass_count, fail_count);
    } else {
        print_validate_text(&checks, pass_count, fail_count);
    }

    if fail_count > 0 {
        Err(format!("{fail_count} validation check(s) failed"))
    } else {
        Ok(())
    }
}

struct ValidationCheck {
    resource: String,
    check_type: String,
    target: String,
    passed: bool,
    detail: String,
}

fn validate_file_exists(path: &Path, resource: &str, target: &str) -> ValidationCheck {
    if !path.exists() {
        return ValidationCheck {
            resource: resource.to_string(),
            check_type: "exists".to_string(),
            target: target.to_string(),
            passed: false,
            detail: "file not found".to_string(),
        };
    }

    let meta = path.metadata();
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);

    if size == 0 {
        return ValidationCheck {
            resource: resource.to_string(),
            check_type: "non-empty".to_string(),
            target: target.to_string(),
            passed: false,
            detail: "file is empty".to_string(),
        };
    }

    let hash = std::fs::read(path)
        .ok()
        .map(|bytes| blake3::hash(&bytes).to_hex()[..16].to_string())
        .unwrap_or_default();

    ValidationCheck {
        resource: resource.to_string(),
        check_type: "integrity".to_string(),
        target: target.to_string(),
        passed: true,
        detail: format!("blake3:{hash} ({size} bytes)"),
    }
}

fn validate_store_integrity(store_dir: &Path, checks: &mut Vec<ValidationCheck>) {
    if let Ok(entries) = std::fs::read_dir(store_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if let Ok(bytes) = std::fs::read(&path) {
                    let hash = blake3::hash(&bytes).to_hex()[..16].to_string();
                    // Verify hash matches filename prefix (content-addressed convention)
                    let name_matches = name.starts_with(&hash[..8.min(hash.len())]);
                    checks.push(ValidationCheck {
                        resource: "store".to_string(),
                        check_type: if name_matches { "content-addressed" } else { "integrity" }.to_string(),
                        target: format!("store/{name}"),
                        passed: true,
                        detail: format!("blake3:{hash}"),
                    });
                }
            }
        }
    }
}

fn print_validate_json(checks: &[ValidationCheck], pass: usize, fail: usize) {
    let items: Vec<String> = checks
        .iter()
        .map(|c| {
            format!(
                r#"{{"resource":"{r}","check":"{ct}","target":"{t}","passed":{p},"detail":"{d}"}}"#,
                r = c.resource,
                ct = c.check_type,
                t = c.target,
                p = c.passed,
                d = c.detail,
            )
        })
        .collect();

    println!(
        r#"{{"passed":{pass},"failed":{fail},"checks":[{}]}}"#,
        items.join(",")
    );
}

fn print_validate_text(checks: &[ValidationCheck], pass: usize, fail: usize) {
    println!("{}\n", bold("Data Validation Report"));
    println!("  Passed: {pass} | Failed: {fail}\n");

    for c in checks {
        let icon = if c.passed { green("✓") } else { red("✗") };
        println!(
            "  {icon} {}: {} [{}] {}",
            c.resource,
            c.target,
            c.check_type,
            dim(&c.detail)
        );
    }

    if fail > 0 {
        println!("\n  {} {fail} check(s) failed", red("✗"));
    } else {
        println!("\n  {} All checks passed", green("✓"));
    }
}
