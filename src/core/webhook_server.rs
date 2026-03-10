//! FJ-3105: Minimal webhook HTTP server using std::net.
//!
//! Accepts POST requests and converts them to [`InfraEvent`] values
//! via the validation pipeline in [`webhook_source`].

use crate::core::types::InfraEvent;
use crate::core::webhook_source::{self, ack_response, WebhookConfig, WebhookRequest};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

/// Start the webhook server on the configured port.
///
/// Blocks the calling thread, accepting connections until `shutdown`
/// is set to `true`. Valid events are forwarded through `sender`.
pub fn run_webhook_server(
    config: &WebhookConfig,
    sender: Sender<InfraEvent>,
    shutdown: Arc<AtomicBool>,
) -> Result<(), String> {
    let addr = format!("127.0.0.1:{}", config.port);
    let listener = TcpListener::bind(&addr).map_err(|e| format!("bind {addr}: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("nonblocking: {e}"))?;

    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, peer)) => {
                handle_connection(stream, &peer.to_string(), config, &sender);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                eprintln!("webhook accept error: {e}");
            }
        }
    }
    Ok(())
}

/// Process a single accepted connection.
fn handle_connection(
    mut stream: std::net::TcpStream,
    peer: &str,
    config: &WebhookConfig,
    sender: &Sender<InfraEvent>,
) {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .ok();
    let buf_size = config.max_body_bytes.min(65536) + 4096;
    let mut buf = vec![0u8; buf_size];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let raw = String::from_utf8_lossy(&buf[..n]);

    match parse_http_to_webhook(&raw, peer) {
        Ok(req) => {
            let resp = dispatch_request(&req, config, sender);
            let _ = stream.write_all(resp.as_bytes());
        }
        Err(resp) => {
            let _ = stream.write_all(resp.as_bytes());
        }
    }
}

/// Validate the request and dispatch to the event pipeline.
/// Returns the HTTP response string.
fn dispatch_request(
    req: &WebhookRequest,
    config: &WebhookConfig,
    sender: &Sender<InfraEvent>,
) -> String {
    let validation = webhook_source::validate_request(config, req);
    if !validation.is_valid() {
        return ack_response(403, &format!("{validation:?}"));
    }

    match webhook_source::request_to_event(req) {
        Ok(event) => {
            let _ = sender.send(event);
            ack_response(200, "accepted")
        }
        Err(e) => ack_response(400, &e),
    }
}

/// Parse raw HTTP text into a [`WebhookRequest`].
///
/// Returns `Err(response)` with a pre-formatted HTTP error on parse failure.
fn parse_http_to_webhook(raw: &str, peer: &str) -> Result<WebhookRequest, String> {
    let (head, body) = split_head_body(raw);
    let mut lines = head.lines();

    // Request line: "POST /webhook HTTP/1.1"
    let request_line = lines
        .next()
        .ok_or_else(|| ack_response(400, "empty request"))?;
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(ack_response(400, "malformed request line"));
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();

    // Headers
    let headers = parse_headers(lines);

    Ok(WebhookRequest {
        method,
        path,
        headers,
        body: body.to_string(),
        source_ip: Some(peer.to_string()),
    })
}

/// Split raw HTTP at the `\r\n\r\n` boundary into head and body.
fn split_head_body(raw: &str) -> (&str, &str) {
    if let Some(pos) = raw.find("\r\n\r\n") {
        (&raw[..pos], &raw[pos + 4..])
    } else if let Some(pos) = raw.find("\n\n") {
        (&raw[..pos], &raw[pos + 2..])
    } else {
        (raw, "")
    }
}

/// Parse header lines into a lowercase-keyed map.
fn parse_headers<'a>(lines: impl Iterator<Item = &'a str>) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc;

    /// Find an available port by binding to :0.
    fn free_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    }

    /// Spin up the server in a background thread and return (port, rx, shutdown).
    fn start_server(config: WebhookConfig) -> (u16, mpsc::Receiver<InfraEvent>, Arc<AtomicBool>) {
        let port = config.port;
        let (tx, rx) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);
        std::thread::spawn(move || {
            let _ = run_webhook_server(&config, tx, shutdown_clone);
        });
        // Give the listener time to bind.
        std::thread::sleep(std::time::Duration::from_millis(100));
        (port, rx, shutdown)
    }

    fn send_raw(port: u16, raw: &str) -> String {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        stream.write_all(raw.as_bytes()).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .ok();
        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        String::from_utf8_lossy(&buf[..n]).to_string()
    }

    fn post_json(port: u16, path: &str, body: &str) -> String {
        let raw = format!(
            "POST {path} HTTP/1.1\r\n\
             Host: localhost\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {body}",
            body.len()
        );
        send_raw(port, &raw)
    }

    #[test]
    fn server_accepts_valid_post() {
        let port = free_port();
        let config = WebhookConfig {
            port,
            ..WebhookConfig::default()
        };
        let (_port, rx, shutdown) = start_server(config);

        let resp = post_json(port, "/webhook", r#"{"action":"deploy"}"#);
        assert!(resp.contains("200 OK"), "response: {resp}");

        let event = rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap();
        assert_eq!(event.payload.get("action").unwrap(), "deploy");

        shutdown.store(true, Ordering::Relaxed);
    }

    #[test]
    fn server_rejects_bad_path() {
        let port = free_port();
        let config = WebhookConfig {
            port,
            ..WebhookConfig::default()
        };
        let (_port, _rx, shutdown) = start_server(config);

        let resp = post_json(port, "/evil", r#"{}"#);
        assert!(resp.contains("403"), "response: {resp}");

        shutdown.store(true, Ordering::Relaxed);
    }

    #[test]
    fn server_rejects_get() {
        let port = free_port();
        let config = WebhookConfig {
            port,
            ..WebhookConfig::default()
        };
        let (_port, _rx, shutdown) = start_server(config);

        let raw = "GET /webhook HTTP/1.1\r\nHost: localhost\r\n\r\n".to_string();
        let resp = send_raw(port, &raw);
        assert!(resp.contains("403"), "response: {resp}");

        shutdown.store(true, Ordering::Relaxed);
    }

    #[test]
    fn server_rejects_invalid_json_body() {
        let port = free_port();
        let config = WebhookConfig {
            port,
            ..WebhookConfig::default()
        };
        let (_port, _rx, shutdown) = start_server(config);

        let resp = post_json(port, "/webhook", "not json");
        assert!(resp.contains("400"), "response: {resp}");

        shutdown.store(true, Ordering::Relaxed);
    }

    #[test]
    fn server_shutdown_flag() {
        let port = free_port();
        let config = WebhookConfig {
            port,
            ..WebhookConfig::default()
        };
        let (tx, _rx) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(true)); // pre-set
        let result = run_webhook_server(&config, tx, shutdown);
        assert!(result.is_ok());
    }

    // -- Unit tests for parse helpers --

    #[test]
    fn parse_http_valid() {
        let raw = "POST /webhook HTTP/1.1\r\n\
                    Host: localhost\r\n\
                    X-Custom: hello\r\n\
                    \r\n\
                    {\"a\":1}";
        let req = parse_http_to_webhook(raw, "1.2.3.4:5678").unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/webhook");
        assert_eq!(req.headers.get("x-custom").unwrap(), "hello");
        assert_eq!(req.body, r#"{"a":1}"#);
        assert_eq!(req.source_ip.as_deref(), Some("1.2.3.4:5678"));
    }

    #[test]
    fn parse_http_empty() {
        let result = parse_http_to_webhook("", "peer");
        assert!(result.is_err());
    }

    #[test]
    fn parse_http_malformed_request_line() {
        let result = parse_http_to_webhook("BADLINE", "peer");
        assert!(result.is_err());
    }

    #[test]
    fn split_head_body_crlf() {
        let (head, body) = split_head_body("HEAD\r\n\r\nBODY");
        assert_eq!(head, "HEAD");
        assert_eq!(body, "BODY");
    }

    #[test]
    fn split_head_body_lf_only() {
        let (head, body) = split_head_body("HEAD\n\nBODY");
        assert_eq!(head, "HEAD");
        assert_eq!(body, "BODY");
    }

    #[test]
    fn split_head_body_no_separator() {
        let (head, body) = split_head_body("HEADONLY");
        assert_eq!(head, "HEADONLY");
        assert_eq!(body, "");
    }

    #[test]
    fn parse_headers_lowercased() {
        let lines = vec!["Content-Type: text/plain", "X-UPPER: VALUE"];
        let headers = parse_headers(lines.into_iter());
        assert_eq!(headers.get("content-type").unwrap(), "text/plain");
        assert_eq!(headers.get("x-upper").unwrap(), "VALUE");
    }

    #[test]
    fn parse_headers_stops_on_blank() {
        let lines = vec!["Key: Val", "", "After: Blank"];
        let headers = parse_headers(lines.into_iter());
        assert_eq!(headers.len(), 1);
    }

    #[test]
    fn dispatch_valid_request() {
        let config = WebhookConfig::default();
        let (tx, rx) = mpsc::channel();
        let req = WebhookRequest {
            method: "POST".into(),
            path: "/webhook".into(),
            headers: HashMap::new(),
            body: r#"{"k":"v"}"#.into(),
            source_ip: None,
        };
        let resp = dispatch_request(&req, &config, &tx);
        assert!(resp.contains("200"));
        let event = rx.try_recv().unwrap();
        assert_eq!(event.payload.get("k").unwrap(), "v");
    }

    #[test]
    fn dispatch_forbidden_path() {
        let config = WebhookConfig::default();
        let (tx, _rx) = mpsc::channel();
        let req = WebhookRequest {
            method: "POST".into(),
            path: "/nope".into(),
            headers: HashMap::new(),
            body: r#"{}"#.into(),
            source_ip: None,
        };
        let resp = dispatch_request(&req, &config, &tx);
        assert!(resp.contains("403"));
    }

    #[test]
    fn dispatch_bad_json() {
        let config = WebhookConfig::default();
        let (tx, _rx) = mpsc::channel();
        let req = WebhookRequest {
            method: "POST".into(),
            path: "/webhook".into(),
            headers: HashMap::new(),
            body: "not json".into(),
            source_ip: None,
        };
        let resp = dispatch_request(&req, &config, &tx);
        assert!(resp.contains("400"));
    }
}
