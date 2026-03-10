//! Example: Webhook event source and server (FJ-3104, FJ-3105)
//!
//! Demonstrates webhook request validation, HMAC signature
//! verification, conversion to InfraEvent, and the webhook_server
//! module for starting/stopping a live listener.
//!
//! ```bash
//! cargo run --example webhook_source
//! ```

use forjar::core::webhook_server;
use forjar::core::webhook_source::{
    ack_response, compute_hmac_hex, parse_json_payload, request_to_event, validate_request,
    WebhookConfig, WebhookRequest,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

fn main() {
    println!("=== Webhook Event Source & Server (FJ-3104 / FJ-3105) ===\n");

    // 1. Default configuration
    let config = WebhookConfig::default();
    println!("1. Default Config:");
    println!("   Port: {}", config.port);
    println!("   Max body: {} bytes", config.max_body_bytes);
    println!("   Allowed paths: {:?}", config.allowed_paths);
    println!(
        "   Secret: {}",
        if config.secret.is_some() {
            "set"
        } else {
            "none"
        }
    );

    // 2. Validate requests
    println!("\n2. Request Validation:");

    let good_req = WebhookRequest {
        method: "POST".into(),
        path: "/webhook".into(),
        headers: HashMap::new(),
        body: r#"{"action":"deploy","env":"production"}"#.into(),
        source_ip: Some("10.0.0.1".into()),
    };
    let result = validate_request(&config, &good_req);
    println!("   POST /webhook (valid body)  → {:?}", result);

    let get_req = WebhookRequest {
        method: "GET".into(),
        path: "/webhook".into(),
        headers: HashMap::new(),
        body: String::new(),
        source_ip: None,
    };
    let result = validate_request(&config, &get_req);
    println!("   GET  /webhook               → {:?}", result);

    let bad_path = WebhookRequest {
        method: "POST".into(),
        path: "/admin/hack".into(),
        headers: HashMap::new(),
        body: "{}".into(),
        source_ip: None,
    };
    let result = validate_request(&config, &bad_path);
    println!("   POST /admin/hack            → {:?}", result);

    // 3. HMAC signature verification
    println!("\n3. HMAC Signature Verification:");
    let secret = "my-webhook-secret";
    let body = r#"{"event":"deploy"}"#;
    let sig = compute_hmac_hex(secret, body);
    println!("   Secret: {secret}");
    println!("   Signature: {}...", &sig[..16]);

    let signed_config = WebhookConfig {
        secret: Some(secret.into()),
        ..WebhookConfig::default()
    };

    let mut signed_req = WebhookRequest {
        method: "POST".into(),
        path: "/webhook".into(),
        headers: HashMap::new(),
        body: body.into(),
        source_ip: None,
    };
    signed_req.headers.insert("x-forjar-signature".into(), sig);
    let result = validate_request(&signed_config, &signed_req);
    println!("   Valid signature   → {:?}", result);

    signed_req
        .headers
        .insert("x-forjar-signature".into(), "bad".into());
    let result = validate_request(&signed_config, &signed_req);
    println!("   Invalid signature → {:?}", result);

    // 4. JSON payload parsing
    println!("\n4. JSON Payload Parsing:");
    let payloads = [
        r#"{"action":"restart","service":"nginx"}"#,
        r#"{"count":42,"tags":["web","prod"]}"#,
    ];
    for body in &payloads {
        match parse_json_payload(body) {
            Ok(kv) => {
                let pairs: Vec<_> = kv.iter().map(|(k, v)| format!("{k}={v}")).collect();
                println!("   {} → {}", body, pairs.join(", "));
            }
            Err(e) => println!("   {} → ERROR: {e}", body),
        }
    }

    // 5. Convert to InfraEvent
    println!("\n5. Request → InfraEvent:");
    let event = request_to_event(&good_req).unwrap();
    println!("   Type: {:?}", event.event_type);
    println!("   Payload:");
    for (k, v) in &event.payload {
        println!("     {k}: {v}");
    }

    // 6. HTTP response formatting
    println!("\n6. Response Formatting:");
    let resp = ack_response(200, "accepted");
    println!("   200: {}", resp.lines().next().unwrap());
    let resp = ack_response(401, "unauthorized");
    println!("   401: {}", resp.lines().next().unwrap());

    // 7. Webhook server: start, receive, and stop
    println!("\n7. Webhook Server (start/receive/stop):");

    // Pick port 0 to let OS assign a free port, but WebhookConfig
    // takes a u16 — use a high ephemeral port instead.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener); // free the port so the server can bind it

    let server_config = WebhookConfig {
        port,
        ..WebhookConfig::default()
    };

    let (tx, rx) = mpsc::channel();
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown);
    let cfg = server_config.clone();

    // Start the server in a background thread
    let handle = std::thread::spawn(move || {
        let _ = webhook_server::run_webhook_server(&cfg, tx, shutdown_clone);
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    println!("   Server started on 127.0.0.1:{port}");

    // Send a webhook request via a raw TCP connection
    {
        use std::io::Write;
        let payload = r#"{"action":"deploy","env":"staging"}"#;
        let raw = format!(
            "POST /webhook HTTP/1.1\r\nHost: localhost\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\r\n{}",
            payload.len(),
            payload
        );
        if let Ok(mut stream) = std::net::TcpStream::connect(format!("127.0.0.1:{port}")) {
            let _ = stream.write_all(raw.as_bytes());
        }
    }

    // Receive the event from the channel
    match rx.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok(ev) => {
            println!("   Received event: type={}", ev.event_type);
            for (k, v) in &ev.payload {
                println!("     {k}: {v}");
            }
        }
        Err(_) => println!("   (no event received — timeout)"),
    }

    // Signal shutdown and wait for the server thread
    shutdown.store(true, Ordering::Relaxed);
    let _ = handle.join();
    println!("   Server stopped.");

    println!("\nDone.");
}
