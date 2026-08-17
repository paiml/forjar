//! GH-247: `forjar verify` — regenerate declared outputs into a scratch tree
//! and report whether they still reproduce.
//!
//! The whole point is the negative guarantee: this never writes a declared
//! output path. See `core::verify` for how that is enforced structurally
//! rather than by convention.

use super::commands::VerifyArgs;
use super::helpers::*;
use crate::core::verify::{verify_resource, Verdict, VerifyOutcome};
use std::path::Path;

/// Exit non-zero if any resource diverged or failed to regenerate.
pub(crate) fn cmd_verify(args: &VerifyArgs, verbose: bool) -> Result<(), String> {
    let VerifyArgs {
        file,
        resource: resource_filter,
        tag: tag_filter,
        state_dir,
        json,
        keep_scratch,
    } = args;
    let (json, keep_scratch) = (*json, *keep_scratch);
    let (resource_filter, tag_filter) = (resource_filter.as_deref(), tag_filter.as_deref());
    let config = parse_and_validate(file)?;

    let scratch_base = std::env::temp_dir().join(format!("forjar-verify-{}", std::process::id()));
    let mut outcomes: Vec<VerifyOutcome> = Vec::new();

    for (id, resource) in &config.resources {
        if resource_filter.is_some_and(|f| id != f) {
            continue;
        }
        if let Some(tag) = tag_filter {
            if !resource.tags.iter().any(|t| t == tag) {
                continue;
            }
        }
        let recorded = recorded_output_hash(state_dir.as_path(), id);
        let scratch = scratch_base.join(id);
        if verbose {
            eprintln!("verify {id}: scratch {}", scratch.display());
        }
        outcomes.push(verify_resource(id, resource, recorded.as_deref(), &scratch));
    }

    if !keep_scratch {
        let _ = std::fs::remove_dir_all(&scratch_base);
    } else if !outcomes.is_empty() {
        eprintln!("scratch retained at {}", scratch_base.display());
    }

    if json {
        print_json(&outcomes);
    } else {
        print_human(&outcomes);
    }

    let failed = outcomes.iter().filter(|o| o.verdict.is_failure()).count();
    if failed > 0 {
        return Err(format!("{failed} resource(s) did not reproduce"));
    }
    Ok(())
}

/// The `output_hash` the last apply recorded for `id`, if any.
///
/// Lives at `resources.<id>.details.output_hash` in a machine's
/// `state.lock.yaml` — NOT at `resources.<id>.output_hash`, which is what the
/// first version of this looked for. The symptom was every resource reporting
/// `skipped (no-recorded-hash)`, which reads like "nothing to do" rather than
/// like a bug, so the end-to-end test is what caught it.
fn recorded_output_hash(state_dir: &Path, id: &str) -> Option<String> {
    let entries = std::fs::read_dir(state_dir).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let lock = entry.path().join("state.lock.yaml");
        if !lock.is_file() {
            continue;
        }
        // `continue`, not `?`: a state dir holds one subdirectory per machine,
        // and bailing on the first unreadable one would silently skip the rest.
        let Ok(text) = std::fs::read_to_string(&lock) else {
            continue;
        };
        let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else {
            continue;
        };
        if let Some(h) = doc
            .get("resources")
            .and_then(|r| r.get(id))
            .and_then(|r| r.get("details"))
            .and_then(|d| d.get("output_hash"))
            .and_then(|v| v.as_str())
        {
            return Some(h.to_string());
        }
    }
    None
}

fn print_human(outcomes: &[VerifyOutcome]) {
    for o in outcomes {
        match &o.verdict {
            Verdict::Reproduced => println!("  reproduced   {}", o.resource_id),
            Verdict::Diverged { recorded, .. } => {
                println!("  DIVERGED     {} (recorded {})", o.resource_id, recorded);
            }
            Verdict::CommandFailed { status } => {
                println!("  FAILED       {} ({status})", o.resource_id);
            }
            Verdict::Skipped(r) => println!("  skipped      {} ({})", o.resource_id, r.as_str()),
        }
    }
    let repro = outcomes
        .iter()
        .filter(|o| o.verdict == Verdict::Reproduced)
        .count();
    let failed = outcomes.iter().filter(|o| o.verdict.is_failure()).count();
    println!(
        "\nVerify: {repro} reproduced, {failed} not reproduced, {} skipped.",
        outcomes.len() - repro - failed
    );
}

/// Machine-readable output, so CI can gate on this without scraping prose.
fn print_json(outcomes: &[VerifyOutcome]) {
    let items: Vec<String> = outcomes
        .iter()
        .map(|o| {
            let extra = match &o.verdict {
                Verdict::Diverged {
                    recorded,
                    regenerated,
                } => format!(
                    r#","recorded":"{recorded}","regenerated":{}"#,
                    regenerated
                        .as_ref()
                        .map_or("null".to_string(), |r| format!("\"{r}\""))
                ),
                Verdict::CommandFailed { status } => {
                    format!(r#","error":"{}""#, status.replace('"', "'"))
                }
                Verdict::Skipped(r) => format!(r#","reason":"{}""#, r.as_str()),
                Verdict::Reproduced => String::new(),
            };
            format!(
                r#"{{"resource":"{}","verdict":"{}"{extra}}}"#,
                o.resource_id,
                o.verdict.as_str()
            )
        })
        .collect();
    println!(
        r#"{{"reproduced":{},"not_reproduced":{},"results":[{}]}}"#,
        outcomes
            .iter()
            .filter(|o| o.verdict == Verdict::Reproduced)
            .count(),
        outcomes.iter().filter(|o| o.verdict.is_failure()).count(),
        items.join(",")
    );
}
