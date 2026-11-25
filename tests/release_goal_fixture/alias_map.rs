//! Committed `pr_ticket_aliases` checker against git history and `roadmap.yaml`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn git_in(repo: &Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git must run");
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn yaml_in(repo: &Path, rel: &str) -> serde_yaml_ng::Value {
    let text = std::fs::read_to_string(repo.join(rel)).expect(rel);
    serde_yaml_ng::from_str(&text).expect(rel)
}

fn normalize_ticket(raw: &str) -> String {
    match raw.strip_prefix("forjar#") {
        Some(n) => format!("GH-{n}"),
        None => raw.to_string(),
    }
}

fn find_frozen_pr_commit<'a>(
    repo: &Path,
    pr: u64,
    frozen: &str,
    log: &'a str,
) -> Result<&'a str, String> {
    let suffix = format!("(#{pr})");
    let hits: Vec<&str> = log
        .lines()
        .filter(|l| l.ends_with(&suffix))
        .filter_map(|l| l.split('\t').next())
        .collect();
    if hits.len() != 1 {
        return Err(format!(
            "PR #{pr} must match one first-parent commit on HEAD, got {hits:?}"
        ));
    }
    let oid = hits[0];
    if git_in(repo, &["merge-base", "--is-ancestor", oid, frozen]).is_none() {
        return Err(format!("PR #{pr} merged after the freeze point"));
    }
    Ok(oid)
}

fn pr_ticket_trailers(repo: &Path, pr: u64, oid: &str) -> Result<Vec<String>, String> {
    let msg = git_in(repo, &["show", "-s", "--format=%B", oid])
        .ok_or_else(|| format!("PR #{pr}: git show {oid} failed"))?;
    Ok(msg
        .lines()
        .filter_map(|l| l.strip_prefix("Pmat-Ticket: "))
        .map(|s| s.trim().to_string())
        .collect())
}

fn ticket_keys(rows: &[serde_yaml_ng::Value], ticket: &str) -> BTreeSet<String> {
    let issue = rows
        .iter()
        .find(|r| r["id"].as_str() == Some(ticket))
        .and_then(|r| r["github_issue"].as_u64());
    let mut keys = BTreeSet::new();
    keys.insert(ticket.to_string());
    if let Some(n) = issue {
        keys.insert(format!("PMAT-{n}"));
    }
    keys
}

fn check_evidence(
    pr: u64,
    oid: &str,
    ticket: &str,
    evidence: &str,
    keys: &BTreeSet<String>,
    trailers: &[String],
) -> Result<(), String> {
    let hit = keys.iter().find_map(|n| {
        let k = trailers.iter().filter(|t| t.as_str() == n.as_str()).count();
        (k > 0).then(|| format!("Pmat-Ticket: {n} ({k} of {})", trailers.len()))
    });
    let want = hit.ok_or_else(|| {
        format!("PR #{pr}: no Pmat-Ticket trailer of {oid} names {ticket}: {trailers:?}")
    })?;
    if !evidence.contains(&want) {
        return Err(format!(
            "PR #{pr}: the evidence {evidence:?} does not state the measured {want:?}"
        ));
    }
    Ok(())
}

fn check_plurality(
    pr: u64,
    ticket: &str,
    keys: &BTreeSet<String>,
    trailers: &[String],
) -> Result<(), String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for t in trailers {
        *counts.entry(normalize_ticket(t)).or_insert(0) += 1;
    }
    let max = counts.values().copied().max().unwrap_or(0);
    let owned: usize = keys.iter().filter_map(|k| counts.get(k)).sum();
    if owned < max {
        return Err(format!(
            "PR #{pr}: {ticket} has {owned} of {} trailers, less than plurality {max}",
            trailers.len()
        ));
    }
    Ok(())
}

fn check_entry(
    repo: &Path,
    entry: &serde_yaml_ng::Value,
    rows: &[serde_yaml_ng::Value],
    frozen: &str,
    log: &str,
) -> Result<bool, String> {
    let pr = entry["pr"].as_u64().ok_or("pr is not a number")?;
    let ticket = entry["ticket"].as_str().ok_or("ticket is not a string")?;
    let evidence = entry["evidence"]
        .as_str()
        .ok_or("evidence is not a string")?;
    let oid = find_frozen_pr_commit(repo, pr, frozen, log)?;
    let trailers = pr_ticket_trailers(repo, pr, oid)?;
    let keys = ticket_keys(rows, ticket);
    check_evidence(pr, oid, ticket, evidence, &keys, &trailers)?;
    check_plurality(pr, ticket, &keys, &trailers)?;
    Ok(oid == frozen)
}

fn parse_freeze_sha(ledger: &serde_yaml_ng::Value) -> Result<&str, String> {
    let frozen = ledger["pr_ticket_aliases_frozen_at"]
        .as_str()
        .ok_or("the alias map declares pr_ticket_aliases_frozen_at")?;
    if frozen.len() != 40 || !frozen.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "the freeze point is a full commit sha, got {frozen:?}"
        ));
    }
    Ok(frozen)
}

fn validate_alias_row(
    entry: &serde_yaml_ng::Value,
    rows: &[serde_yaml_ng::Value],
    seen: &mut BTreeSet<u64>,
) -> Result<(), String> {
    let pr = entry["pr"].as_u64().ok_or("pr is a number")?;
    let ticket = entry["ticket"].as_str().ok_or("ticket is a string")?;
    if !seen.insert(pr) {
        return Err(format!("PR #{pr} is mapped twice"));
    }
    if entry["evidence"].as_str().unwrap_or("").trim().is_empty() {
        return Err(format!("PR #{pr}: no evidence"));
    }
    if !rows.iter().any(|r| r["id"].as_str() == Some(ticket)) {
        return Err(format!("PR #{pr}: {ticket} is no roadmap row"));
    }
    Ok(())
}

fn validate_aliases(
    aliases: &[serde_yaml_ng::Value],
    rows: &[serde_yaml_ng::Value],
) -> Result<(), String> {
    if aliases.is_empty() {
        return Err("the alias map is empty".to_string());
    }
    let mut seen = BTreeSet::new();
    for entry in aliases {
        validate_alias_row(entry, rows, &mut seen)?;
    }
    Ok(())
}

fn validate_freeze_commit(repo: &Path, frozen: &str) -> Result<(), String> {
    if git_in(
        repo,
        &["rev-parse", "--verify", &format!("{frozen}^{{commit}}")],
    )
    .is_none()
    {
        return Err(format!("the freeze point {frozen} resolves to no commit"));
    }
    if git_in(repo, &["merge-base", "--is-ancestor", frozen, "HEAD"]).is_none() {
        return Err(format!("the freeze point {frozen} is not history of HEAD"));
    }
    Ok(())
}

pub(crate) fn check_alias_map(repo: &Path) -> Result<(), String> {
    let ledger = yaml_in(repo, "docs/roadmaps/releases.yaml");
    let roadmap = yaml_in(repo, "docs/roadmaps/roadmap.yaml");
    let rows = roadmap["roadmap"].as_sequence().ok_or("roadmap rows")?;
    let frozen = parse_freeze_sha(&ledger)?;
    let aliases = ledger["pr_ticket_aliases"].as_sequence().ok_or("aliases")?;
    validate_aliases(aliases, rows)?;
    validate_freeze_commit(repo, frozen)?;
    let log = git_in(
        repo,
        &["log", "--first-parent", "--format=%H%x09%s", "HEAD"],
    )
    .ok_or("git log failed")?;
    let mut newest = false;
    for a in aliases {
        newest |= check_entry(repo, a, rows, frozen, &log)?;
    }
    if !newest {
        return Err(
            "the freeze point is not the merge commit of an aliased PR: it was moved".to_string(),
        );
    }
    Ok(())
}
