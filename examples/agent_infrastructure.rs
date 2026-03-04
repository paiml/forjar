//! Agent infrastructure and pforge integration demonstration.
//!
//! Shows forjar's agent management capabilities:
//! structured logging, progress tracking, and LSP server.

fn main() {
    println!("=== Forjar Agent Infrastructure Demo ===\n");

    demo_structured_logging();
    demo_progress_tracking();
    demo_lsp_capabilities();

    println!("\n=== Agent infrastructure demo complete ===");
}

fn demo_structured_logging() {
    use forjar::cli::structured_log::*;

    println!("--- Structured Logging ---");

    set_level(Level::Debug);
    set_json(false);

    let events: Vec<(Level, &str, Vec<(&str, &str)>)> = vec![
        (Level::Info, "Starting apply", vec![("machine", "web-01")]),
        (Level::Debug, "Resolving deps", vec![("count", "5")]),
        (Level::Warn, "Stale data source", vec![("age_hours", "48")]),
        (
            Level::Info,
            "Apply complete",
            vec![("changed", "3"), ("unchanged", "2")],
        ),
    ];

    for (level, msg, fields) in &events {
        let field_str: Vec<String> = fields.iter().map(|(k, v)| format!("{k}={v}")).collect();
        println!("  [{:?}] {} {}", level, msg, field_str.join(" "));
    }

    set_json(true);
    println!("\n  JSON mode sample:");
    println!("  {{\"level\":\"info\",\"msg\":\"Apply complete\",\"changed\":3}}");
    set_json(false);
    println!();
}

fn demo_progress_tracking() {
    use forjar::cli::progress::*;

    println!("--- Progress Tracking ---");

    // Track overall progress
    let mut tracker = track_progress(5);
    println!("  Tracker: 0/{} resources", 5);

    tracker.complete();
    tracker.complete();
    tracker.fail();
    tracker.complete();
    tracker.skip();

    let report = tracker.report();
    println!(
        "  Result: {} completed, {} failed, {} skipped ({:.1}s)",
        report.completed, report.failed, report.skipped, report.elapsed_secs
    );

    // Progress bar
    let mut bar = ProgressBar::new(10, "Applying resources");
    bar.set(3);
    bar.inc();
    bar.set(10);
    bar.finish();
    println!("  Progress bar: 10/10 complete");
    println!();
}

fn demo_lsp_capabilities() {
    use forjar::cli::lsp::*;

    println!("--- LSP Server Capabilities ---");

    let mut server = LspServer::new();

    // Initialize
    let init_msg = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": "file:///home/user/infra" }
    });
    let resp = server.handle_message(&init_msg).unwrap();
    let server_name = resp.pointer("/result/serverInfo/name").unwrap();
    println!("  Server: {}", server_name);
    println!(
        "  Root: {}",
        server.root_uri.as_deref().unwrap_or("none")
    );

    // Open a document
    let doc = "machines:\n  - name: web-01\nresources:\n  - name: nginx\n    type: package\n    ensure: present";
    let open_msg = serde_json::json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": { "textDocument": { "uri": "file:///t.yaml", "text": doc } }
    });
    server.handle_message(&open_msg);
    println!("  Document opened ({} bytes)", doc.len());

    // Validate
    let diags = server.validate_document("file:///t.yaml");
    println!("  Diagnostics: {} (0 = valid YAML)", diags.len());

    // Get completions at top level
    let comp_msg = serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": "file:///t.yaml" },
            "position": { "line": 0, "character": 0 }
        }
    });
    let comp_resp = server.handle_message(&comp_msg).unwrap();
    let items = comp_resp
        .pointer("/result")
        .unwrap()
        .as_array()
        .unwrap();
    println!("  Top-level completions: {}", items.len());
    for item in items.iter().take(3) {
        println!("    - {}: {}", item["label"], item["detail"]);
    }

    // Validate bad YAML
    let bad_diags = validate_yaml("ensure: bogus_value");
    println!("\n  Validation of 'ensure: bogus_value':");
    for d in &bad_diags {
        println!(
            "    [{:?}] line {}: {}",
            d.severity, d.line, d.message
        );
    }
    println!();
}
