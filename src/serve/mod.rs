//! `forjar serve` — the verb registry over HTTP.
//!
//! Routes:
//!
//! | method | path              | meaning                          |
//! |--------|-------------------|----------------------------------|
//! | GET    | `/health`         | liveness and verb count          |
//! | GET    | `/v1/verbs`       | the whole catalogue with schemas |
//! | GET    | `/v1/verbs/{name}`| one verb's schema                |
//! | POST   | `/v1/{name}`      | invoke, params as the JSON body  |
//!
//! Every route above is derived. Adding a verb to `Commands` publishes it here
//! with no edit to this module.

pub mod http;
pub mod router;

use crate::verb::VerbCtx;
use router::ServeConfig;
use std::net::{TcpListener, TcpStream};

/// Run the HTTP server until the process is stopped.
///
/// # Errors
///
/// A string describing why the listener could not be bound.
pub fn serve(host: &str, port: u16, read_only: bool) -> Result<(), String> {
    // Build the registry before binding. It is expensive and needs a large
    // stack (see `verb::derive`); doing it here means a derivation failure is a
    // startup failure rather than a 500 on the first request, and no connection
    // thread ever pays the cost.
    let verb_count = crate::verb::registry().len();

    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr).map_err(|e| format!("bind {addr}: {e}"))?;

    let cfg = ServeConfig {
        read_only,
        ctx: VerbCtx::current().map_err(|e| e.to_string())?,
    };

    eprintln!(
        "forjar serve: listening on http://{addr} — {verb_count} verbs{}",
        if read_only { " (read-only)" } else { "" }
    );

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let cfg = cfg.clone();
                // One thread per connection, detached. A panic in a handler
                // takes down only that connection's thread.
                std::thread::spawn(move || handle(s, &cfg));
            }
            Err(e) => eprintln!("forjar serve: accept failed: {e}"),
        }
    }
    Ok(())
}

/// Serve one connection, then close it.
fn handle(mut stream: TcpStream, cfg: &ServeConfig) {
    let peer = stream.try_clone();
    let (status, body) = match http::read_request(&stream) {
        Ok(req) => router::route(&req, cfg),
        Err(e) => (
            400,
            serde_json::json!({
                "error": { "kind": "malformed", "message": e, "exit_code": 1 }
            }),
        ),
    };
    // Read and write halves must be separate handles; the reader consumed one.
    if let Ok(mut w) = peer {
        let _ = http::write_json(&mut w, status, &body);
    } else {
        let _ = http::write_json(&mut stream, status, &body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_a_privileged_port_fails_with_a_named_address() {
        // Port 1 is privileged; the error must say which address failed, or an
        // operator debugging a bind failure has nothing to go on.
        let e = serve("127.0.0.1", 1, false).unwrap_err();
        assert!(e.contains("127.0.0.1:1"), "{e}");
    }

    #[test]
    fn binding_an_unresolvable_host_fails_rather_than_defaulting() {
        // A silent fallback to 0.0.0.0 on a bad host would expose the server.
        let e = serve("no.such.host.invalid", 8737, false).unwrap_err();
        assert!(e.contains("no.such.host.invalid"), "{e}");
    }
}
