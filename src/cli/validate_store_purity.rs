//! FJ-1306 / FJ-1329: Store purity and reproducibility score validation.
//!
//! - `--check-recipe-purity` — report per-resource purity levels
//! - `--check-reproducibility-score` — output 0-100 reproducibility score

use crate::core::store::purity::{classify, level_label, recipe_purity, PurityLevel, PuritySignals};
use crate::core::store::repro_score::{compute_score, grade, ReproInput};
use std::path::Path;

/// Extracted purity data for all resources.
struct PurityExtract {
    resources: Vec<(String, PurityLevel, Vec<String>)>,
    recipe_level: PurityLevel,
}

/// `forjar validate --check-recipe-purity`
///
/// Parses the config, classifies each resource's purity level, and reports
/// the aggregate recipe purity with per-resource breakdown.
pub(crate) fn cmd_validate_check_recipe_purity(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let PurityExtract { resources, recipe_level } = extract_purity(file)?;

    if json {
        let j = serde_json::json!({
            "recipe_purity": format!("{:?}", recipe_level),
            "recipe_purity_level": recipe_level as u8,
            "resources": resources.iter().map(|(name, level, reasons)| {
                serde_json::json!({
                    "name": name,
                    "purity": format!("{:?}", level),
                    "level": *level as u8,
                    "reasons": reasons,
                })
            }).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&j).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Recipe purity: {}", level_label(recipe_level));
        for (name, level, reasons) in &resources {
            println!("  {name}: {}", level_label(*level));
            for r in reasons {
                println!("    - {r}");
            }
        }
    }
    Ok(())
}

/// `forjar validate --check-reproducibility-score`
///
/// Computes a 0-100 reproducibility score weighted by purity (50%),
/// store coverage (30%), and lock coverage (20%).
pub(crate) fn cmd_validate_check_reproducibility_score(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let inputs = extract_repro_inputs(file)?;
    let score = compute_score(&inputs);

    if json {
        let j = serde_json::json!({
            "composite": score.composite,
            "grade": grade(score.composite),
            "purity_score": score.purity_score,
            "store_score": score.store_score,
            "lock_score": score.lock_score,
            "resources": score.resources,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&j).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!(
            "Reproducibility score: {:.0}/100 (grade {})",
            score.composite,
            grade(score.composite)
        );
        println!(
            "  Purity: {:.0} | Store: {:.0} | Lock: {:.0}",
            score.purity_score, score.store_score, score.lock_score
        );
        for r in &score.resources {
            println!(
                "  {}: {:.0} ({:?}{}{})",
                r.name,
                r.score,
                r.purity,
                if r.has_store { " +store" } else { "" },
                if r.has_lock_pin { " +lock" } else { "" },
            );
        }
    }
    Ok(())
}

/// Extract purity classification for all resources in a config.
fn extract_purity(file: &Path) -> Result<PurityExtract, String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("read {}: {e}", file.display()))?;
    let doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("parse {}: {e}", file.display()))?;

    let resources = doc
        .get("resources")
        .and_then(|r| r.as_mapping())
        .ok_or_else(|| "no resources section found".to_string())?;

    let mut results = Vec::new();
    let mut levels = Vec::new();

    for (key, val) in resources {
        let name = key.as_str().unwrap_or("").to_string();
        let signals = PuritySignals {
            has_version: val.get("version").is_some(),
            has_store: val.get("store").and_then(|v| v.as_bool()).unwrap_or(false),
            has_sandbox: val.get("sandbox").is_some(),
            has_curl_pipe: detect_curl_pipe(val),
            dep_levels: vec![],
        };
        let result = classify(&name, &signals);
        levels.push(result.level);
        results.push((result.name, result.level, result.reasons));
    }

    results.sort_by(|a, b| a.0.cmp(&b.0));
    let recipe_level = recipe_purity(&levels);
    Ok(PurityExtract {
        resources: results,
        recipe_level,
    })
}

/// Extract reproducibility scoring inputs from a config.
fn extract_repro_inputs(file: &Path) -> Result<Vec<ReproInput>, String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("read {}: {e}", file.display()))?;
    let doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("parse {}: {e}", file.display()))?;

    let resources = doc
        .get("resources")
        .and_then(|r| r.as_mapping())
        .ok_or_else(|| "no resources section found".to_string())?;

    // Check for lock file to determine lock pin coverage
    let lock_path = file
        .parent()
        .unwrap_or(Path::new("."))
        .join("forjar.inputs.lock.yaml");
    let lock_pins = if lock_path.exists() {
        std::fs::read_to_string(&lock_path).unwrap_or_default()
    } else {
        String::new()
    };

    let mut inputs = Vec::new();
    for (key, val) in resources {
        let name = key.as_str().unwrap_or("").to_string();
        let signals = PuritySignals {
            has_version: val.get("version").is_some(),
            has_store: val.get("store").and_then(|v| v.as_bool()).unwrap_or(false),
            has_sandbox: val.get("sandbox").is_some(),
            has_curl_pipe: detect_curl_pipe(val),
            dep_levels: vec![],
        };
        let result = classify(&name, &signals);
        let has_lock_pin = lock_pins.contains(&name);

        inputs.push(ReproInput {
            name,
            purity: result.level,
            has_store: signals.has_store,
            has_lock_pin,
        });
    }
    inputs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(inputs)
}

/// Detect curl|bash patterns in resource values.
fn detect_curl_pipe(val: &serde_yaml_ng::Value) -> bool {
    let s = serde_yaml_ng::to_string(val).unwrap_or_default();
    (s.contains("curl") && s.contains("bash")) || (s.contains("wget") && s.contains("sh"))
}
