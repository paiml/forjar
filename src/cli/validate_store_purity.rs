//! FJ-1306 / FJ-1329: Store purity and reproducibility score validation.
//!
//! - `--check-recipe-purity` — report per-resource purity levels
//! - `--check-reproducibility-score` — output 0-100 reproducibility score

use crate::core::store::purity::{
    classify, level_label, recipe_purity, PurityLevel, PuritySignals,
};
use crate::core::store::repro_score::{compute_score, grade, ReproInput};
use std::path::Path;

/// Extracted purity data for all resources.
struct PurityExtract {
    resources: Vec<(String, PurityLevel, Vec<String>)>,
    recipe_level: PurityLevel,
}

/// Accepted `--min-purity` values, in worst-to-best order.
///
/// Lives beside [`parse_min_purity`] so the clap value_parser and the parser
/// cannot disagree about what is accepted — a mismatch would either reject a
/// documented level or accept one nothing handles.
pub(crate) const PURITY_LEVELS: [&str; 4] = ["pure", "pinned", "constrained", "impure"];

/// Parse a `--min-purity` value into a level.
fn parse_min_purity(s: &str) -> Result<PurityLevel, String> {
    match s.to_ascii_lowercase().as_str() {
        "pure" => Ok(PurityLevel::Pure),
        "pinned" => Ok(PurityLevel::Pinned),
        "constrained" => Ok(PurityLevel::Constrained),
        "impure" => Ok(PurityLevel::Impure),
        other => Err(format!(
            "unknown purity level `{other}` (expected pure, pinned, constrained or impure)"
        )),
    }
}

/// `forjar validate --check-recipe-purity [--min-purity LEVEL]`
///
/// Parses the config, classifies each resource's purity level, and reports
/// the aggregate recipe purity with per-resource breakdown.
///
/// GH-241: with `--min-purity` this becomes a gate — a recipe worse than the
/// threshold exits non-zero. Without it the command reports and exits 0, which
/// is what it always did and what its help text now says plainly. It reports;
/// it does not enforce unless you ask it to.
pub(crate) fn cmd_validate_check_recipe_purity(
    file: &Path,
    json: bool,
    min_purity: Option<&str>,
) -> Result<(), String> {
    let min_level = min_purity.map(parse_min_purity).transpose()?;
    let PurityExtract {
        resources,
        recipe_level,
    } = extract_purity(file)?;

    // Worse purity is a HIGHER discriminant (Pure=0 .. Impure=3).
    let pass = min_level.is_none_or(|min| recipe_level <= min);

    if json {
        let j = serde_json::json!({
            "recipe_purity": format!("{:?}", recipe_level),
            "recipe_purity_level": recipe_level as u8,
            "min_purity": min_level.map(|l| format!("{l:?}")),
            "pass": pass,
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

    if pass {
        Ok(())
    } else {
        let min = min_level.expect("pass is only false when a minimum was set");
        Err(format!(
            "recipe purity {} is worse than the required minimum {}",
            level_label(recipe_level),
            level_label(min)
        ))
    }
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

    // GH-241: classify in dependency order and feed each resource its
    // dependencies' RESOLVED levels.
    //
    // This was `dep_levels: vec![]` — unconditionally, at the only production
    // call site. `classify` implements the documented monotonicity invariant
    // (`final_level = max(own_level, max(dep_levels))`), it is unit-tested, and
    // then `max(dep_levels)` was always `None`, so the rule never fired on a
    // real recipe. A Pure resource depending on an Impure one reported Pure.
    //
    // Resolving in topological order makes the propagation transitive for free:
    // by the time a resource is classified, each of its dependencies already
    // carries the max of its own subtree.
    let mut own: Vec<(String, serde_yaml_ng::Value)> = Vec::new();
    let mut deps: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (key, val) in resources {
        let name = key.as_str().unwrap_or("").to_string();
        deps.insert(name.clone(), depends_on_of(val));
        own.push((name, val.clone()));
    }

    let order = purity_classification_order(&own, &deps);
    let mut resolved: std::collections::BTreeMap<String, PurityLevel> =
        std::collections::BTreeMap::new();
    let mut results = Vec::new();
    let mut levels = Vec::new();

    for name in order {
        let Some((_, val)) = own.iter().find(|(n, _)| *n == name) else {
            continue;
        };
        let dep_levels = deps
            .get(&name)
            .map(|ds| ds.iter().filter_map(|d| resolved.get(d).copied()).collect())
            .unwrap_or_default();
        let signals = PuritySignals {
            has_version: val.get("version").is_some(),
            has_store: val.get("store").and_then(|v| v.as_bool()).unwrap_or(false),
            has_sandbox: val.get("sandbox").is_some(),
            has_curl_pipe: detect_curl_pipe(val),
            dep_levels,
        };
        let result = classify(&name, &signals);
        resolved.insert(name, result.level);
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
/// Read a resource's `depends_on` edges from the raw YAML.
///
/// Accepts both the list form and the single-string form, matching what the
/// parser accepts — reading only the list form here would silently drop edges
/// and reintroduce the always-Pure bug for those recipes.
fn depends_on_of(val: &serde_yaml_ng::Value) -> Vec<String> {
    match val.get("depends_on") {
        Some(serde_yaml_ng::Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(serde_yaml_ng::Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    }
}

/// Order resources so every resource is classified after its dependencies.
///
/// Kahn's algorithm over the `depends_on` edges. Any resource left over — a
/// cycle, or an edge to a name that is not a resource — is appended at the end
/// rather than dropped: a cycle is the parser's error to report, and silently
/// omitting a resource from a purity report would understate the recipe's
/// worst level, which is the failure direction that matters here.
fn purity_classification_order(
    own: &[(String, serde_yaml_ng::Value)],
    deps: &std::collections::BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let names: std::collections::BTreeSet<&str> = own.iter().map(|(n, _)| n.as_str()).collect();
    let mut remaining: std::collections::BTreeMap<&str, std::collections::BTreeSet<&str>> = own
        .iter()
        .map(|(n, _)| {
            let pending = deps
                .get(n)
                .map(|ds| {
                    ds.iter()
                        .filter(|d| names.contains(d.as_str()))
                        .map(String::as_str)
                        .collect()
                })
                .unwrap_or_default();
            (n.as_str(), pending)
        })
        .collect();

    let mut order = Vec::with_capacity(own.len());
    while !remaining.is_empty() {
        let ready: Vec<&str> = remaining
            .iter()
            .filter(|(_, pending)| pending.is_empty())
            .map(|(n, _)| *n)
            .collect();
        if ready.is_empty() {
            // Cycle or unresolvable edge — emit the rest in a stable order.
            order.extend(remaining.keys().map(|n| (*n).to_string()));
            break;
        }
        for n in &ready {
            remaining.remove(n);
        }
        for pending in remaining.values_mut() {
            for n in &ready {
                pending.remove(n);
            }
        }
        order.extend(ready.into_iter().map(str::to_string));
    }
    order
}

fn detect_curl_pipe(val: &serde_yaml_ng::Value) -> bool {
    let s = serde_yaml_ng::to_string(val).unwrap_or_default();
    (s.contains("curl") && s.contains("bash")) || (s.contains("wget") && s.contains("sh"))
}
