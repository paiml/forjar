//! FJ-3109: `forjar rules serve` — the webhook receiver's entry point.
//!
//! # Why this exists
//!
//! `run_webhook_server` had ZERO non-test callers. No `Commands` variant reached
//! it, so the receiver could not be started by anyone, which meant it could not be
//! dogfooded and no sender could ever exercise it. That is how it accumulated a
//! wrong MAC algorithm, no HTTP framing, two fail-open defaults and an
//! unauthenticated-DoS path while carrying ~35 passing tests: every one of them
//! called the functions directly.
//!
//! # ⚠️ Actions are NOT executed here
//!
//! Accepted events are evaluated against the rulebook and the matching actions are
//! REPORTED. They are deliberately not run. `rulebook_template::expand_action`
//! substitutes attacker-controlled payload keys into `RulebookAction.script` with
//! `String::replace` and no shell quoting, so wiring an executor to a network
//! listener would turn an inbound request into command execution.
//!
//! That injection is currently unreachable — `expand_action` has no callers,
//! and while `cli::trigger` DOES now execute a rulebook's actions (see
//! `cli::trigger_exec`), it deliberately executes the action text verbatim and
//! never calls `expand_action`, so no payload key reaches a shell. It must stay
//! that way until the quoting is fixed. Receiver authentication, freshness and
//! idempotency landing FIRST is the sequencing gate, not a preference.
//!
//! Note the trust asymmetry that makes `trigger` executing acceptable while
//! this receiver still must not: `trigger`'s rulebook and `--payload` come from
//! the local operator, the same trust level as the `command:` in their own
//! `forjar.yaml`. This listener's events come off the network.

use crate::core::rules_runtime;
use crate::core::types::{CooldownTracker, InfraEvent, RulebookConfig};
use crate::core::webhook_server::run_webhook_server;
use crate::core::webhook_source::WebhookConfig;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Assemble a [`WebhookConfig`] from CLI arguments.
#[allow(clippy::too_many_arguments)]
pub fn build_config(
    bind: String,
    port: u16,
    secret_file: Option<&Path>,
    paths: Vec<String>,
    allow_unauthenticated: bool,
    tls_terminated_upstream: bool,
    tolerance_secs: u64,
) -> Result<WebhookConfig, String> {
    let secret = match secret_file {
        None => None,
        Some(p) => {
            let raw = std::fs::read_to_string(p)
                .map_err(|e| format!("read secret file {}: {e}", p.display()))?;
            // Trim: an editor's trailing newline would otherwise become part of
            // the key, and every signature would fail with no clue why.
            let trimmed = raw.trim().to_string();
            if trimmed.is_empty() {
                return Err(format!("secret file {} is empty", p.display()));
            }
            Some(trimmed)
        }
    };

    Ok(WebhookConfig {
        bind,
        port,
        secret,
        allowed_paths: paths,
        allow_unauthenticated,
        tls_terminated_upstream,
        signature_tolerance_secs: tolerance_secs,
        machine: Some(hostname_or_local()),
        ..WebhookConfig::default()
    })
}

/// Best-effort local machine name for event attribution.
fn hostname_or_local() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "local".to_string())
}

/// Load and validate the rulebook before binding anything.
pub fn load_rulebook(file: &Path) -> Result<RulebookConfig, String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("read {}: {e}", file.display()))?;
    let issues = crate::core::rules_engine::validate_rulebook_yaml(&content)?;
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == crate::core::rules_engine::IssueSeverity::Error)
        .collect();
    if !errors.is_empty() {
        let msgs: Vec<String> = errors
            .iter()
            .map(|i| format!("  {}: {}", i.rulebook, i.message))
            .collect();
        return Err(format!("rulebook validation failed:\n{}", msgs.join("\n")));
    }
    serde_yaml_ng::from_str(&content).map_err(|e| format!("parse rulebook: {e}"))
}

/// Print the configuration that WOULD be served, without binding.
pub fn print_check(config: &WebhookConfig, rulebooks: usize, json: bool) {
    let authenticated = config.secret.is_some();
    if json {
        let out = serde_json::json!({
            "check": true,
            "bind": config.bind,
            "port": config.port,
            "authenticated": authenticated,
            "allowed_paths": config.allowed_paths,
            "signature_tolerance_secs": config.signature_tolerance_secs,
            "rulebooks": rulebooks,
            "actions_executed": false,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        println!("Webhook receiver configuration OK");
        println!("  Bind:      {}:{}", config.bind, config.port);
        println!(
            "  Auth:      {}",
            if authenticated {
                "HMAC-SHA256 (X-Forjar-Signature: t=…,v1=…; X-Hub-Signature-256 accepted)"
            } else {
                "NONE (--allow-unauthenticated)"
            }
        );
        println!("  Paths:     {}", config.allowed_paths.join(", "));
        println!("  Tolerance: {}s", config.signature_tolerance_secs);
        println!("  Rulebooks: {rulebooks}");
        println!("  Actions:   reported, NOT executed (see module docs)");
    }
}

/// Run the receiver, evaluating each accepted event against the rulebook.
pub fn serve(config: WebhookConfig, rulebook: RulebookConfig, json: bool) -> Result<(), String> {
    // Fail before binding, so a bad config never leaves a socket open.
    config.validate_startup()?;

    let (tx, rx) = std::sync::mpsc::channel::<InfraEvent>();
    let shutdown = Arc::new(AtomicBool::new(false));
    let server_config = config.clone();
    let server_shutdown = Arc::clone(&shutdown);

    let handle =
        std::thread::spawn(move || run_webhook_server(&server_config, tx, server_shutdown));

    eprintln!(
        "forjar: webhook receiver listening on {}:{} ({} path(s), {})",
        config.bind,
        config.port,
        config.allowed_paths.len(),
        if config.secret.is_some() {
            "signed"
        } else {
            "UNAUTHENTICATED"
        }
    );

    let mut tracker = CooldownTracker::default();
    // Ends when the server thread drops the sender.
    for event in rx {
        let results = rules_runtime::evaluate_event(&event, &rulebook, &mut tracker);
        let fired: Vec<_> = results
            .iter()
            .filter(|r| !r.cooldown_blocked && !r.disabled && !r.actions.is_empty())
            .collect();
        report(&event, &fired, json);
    }

    shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
    match handle.join() {
        Ok(result) => result,
        Err(_) => Err("webhook server thread panicked".to_string()),
    }
}

/// Report what an event matched. Actions are named, never run.
fn report(event: &InfraEvent, fired: &[&rules_runtime::EvalResult], json: bool) {
    if json {
        let out = serde_json::json!({
            "event": format!("{:?}", event.event_type),
            "timestamp": event.timestamp,
            "machine": event.machine,
            "event_id": event.payload.get("_event_id"),
            "path": event.payload.get("_path"),
            "fired": fired.iter().map(|r| serde_json::json!({
                "rulebook": r.rulebook,
                "actions": r.actions.len(),
            })).collect::<Vec<_>>(),
            "actions_executed": false,
        });
        println!("{}", serde_json::to_string(&out).unwrap_or_default());
    } else {
        println!(
            "event {:?} at {} path={} → {} rulebook(s) matched",
            event.event_type,
            event.timestamp,
            event.payload.get("_path").map_or("-", String::as_str),
            fired.len()
        );
        for r in fired {
            println!(
                "  {} → {} action(s) (not executed)",
                r.rulebook,
                r.actions.len()
            );
        }
    }
}
