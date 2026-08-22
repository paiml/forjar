//! FJ-3105: webhook HTTP server.
//!
//! # Concurrency
//!
//! Connections used to be handled INLINE on the accept loop behind a 5s read
//! timeout, so one client that connected and sent nothing stalled every other
//! delivery. Measured: a single idle socket delayed a legitimate signed request by
//! 5383ms, entirely upstream of `validate_request` — so the secret was no defence.
//!
//! Now a bounded pool of worker threads drains a bounded queue. Bounded rather
//! than thread-per-connection so an attacker cannot spawn threads, and a fixed
//! pool rather than tokio because this is a loopback endpoint taking a handful of
//! deliveries an hour: a synchronous accept loop with N workers is easier to
//! reason about, and easier to state as a contract, than dragging an async runtime
//! into it.

use crate::core::types::InfraEvent;
use crate::core::webhook_http::{self, ReadOutcome};
use crate::core::webhook_sig::{unix_now, ReplayGuard};
use crate::core::webhook_source::{
    self, validate_request_at, ValidationResult, WebhookConfig, WebhookRequest, SIG_HEADER,
};
use std::collections::HashMap;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Worker threads handling connections.
const WORKERS: usize = 4;
/// Queued connections awaiting a worker before we shed load with 503.
const QUEUE_DEPTH: usize = 32;
/// Whole-connection deadline.
const CONN_DEADLINE: Duration = Duration::from_secs(5);
/// Retained replay entries.
const REPLAY_CAPACITY: usize = 4096;

/// Shared state each worker needs.
struct Shared {
    config: WebhookConfig,
    sender: Sender<InfraEvent>,
    replay: Mutex<ReplayGuard>,
}

/// Start the webhook server, blocking until `shutdown` is set.
///
/// Returns `Err` without binding if the configuration would expose an
/// unauthenticated or plaintext endpoint.
pub fn run_webhook_server(
    config: &WebhookConfig,
    sender: Sender<InfraEvent>,
    shutdown: Arc<AtomicBool>,
) -> Result<(), String> {
    config.validate_startup()?;

    let addr = format!("{}:{}", config.bind, config.port);
    let listener = TcpListener::bind(&addr).map_err(|e| format!("bind {addr}: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("nonblocking: {e}"))?;

    let shared = Arc::new(Shared {
        config: config.clone(),
        sender,
        replay: Mutex::new(ReplayGuard::new(
            config.signature_tolerance_secs,
            REPLAY_CAPACITY,
        )),
    });

    let (tx, rx) = std::sync::mpsc::sync_channel::<(TcpStream, String)>(QUEUE_DEPTH);
    let rx = Arc::new(Mutex::new(rx));
    let mut handles = Vec::with_capacity(WORKERS);
    for _ in 0..WORKERS {
        let rx = Arc::clone(&rx);
        let shared = Arc::clone(&shared);
        handles.push(std::thread::spawn(move || {
            loop {
                // Hold the lock only to dequeue, never while serving.
                let next = {
                    let guard = match rx.lock() {
                        Ok(g) => g,
                        Err(_) => break,
                    };
                    guard.recv()
                };
                match next {
                    Ok((stream, peer)) => serve(stream, &peer, &shared),
                    Err(_) => break, // sender dropped: shutting down
                }
            }
        }));
    }

    accept_loop(&listener, &tx, &shutdown);

    drop(tx);
    for h in handles {
        let _ = h.join();
    }
    Ok(())
}

/// Accept connections and hand them to the pool, shedding load when it is full.
fn accept_loop(
    listener: &TcpListener,
    tx: &SyncSender<(TcpStream, String)>,
    shutdown: &Arc<AtomicBool>,
) {
    let mut backoff = Duration::from_millis(50);
    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, peer)) => {
                backoff = Duration::from_millis(50);
                if let Err(TrySendError::Full((mut stream, _))) =
                    tx.try_send((stream, peer.to_string()))
                {
                    // Shed rather than queue without limit, and SAY so — silently
                    // dropping would look identical to a lost packet.
                    let _ = stream.write_all(&webhook_http::response(503, "queue_full"));
                    let _ = stream.flush();
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                // Back off. A persistent accept error (EMFILE) used to spin this
                // loop at full tilt with no delay.
                eprintln!("webhook accept error: {e}");
                std::thread::sleep(backoff);
                backoff = (backoff * 2).min(Duration::from_secs(5));
            }
        }
    }
}

/// Read, validate and dispatch one connection.
fn serve(mut stream: TcpStream, peer: &str, shared: &Shared) {
    // Propagated, not `.ok()`-swallowed: without a read timeout the deadline below
    // cannot be enforced and a worker could block forever.
    if let Err(e) = stream.set_read_timeout(Some(CONN_DEADLINE)) {
        eprintln!("webhook: set_read_timeout failed for {peer}: {e}");
        let _ = stream.write_all(&webhook_http::response(500, "internal_error"));
        return;
    }

    let outcome =
        webhook_http::read_request(&mut stream, shared.config.max_body_bytes, CONN_DEADLINE);

    let reply = match outcome {
        ReadOutcome::Empty => return, // nothing sent; nothing owed
        ReadOutcome::Rejected { status, code } => webhook_http::response(status, code),
        ReadOutcome::Complete {
            method,
            path,
            headers,
            body,
        } => {
            let request = WebhookRequest {
                method,
                path,
                headers: headers.into_iter().collect::<HashMap<_, _>>(),
                body,
                // Address only. The old code stored `ip:port`, so any documented
                // IP match in a rulebook could never fire.
                source_ip: Some(peer.rsplit_once(':').map_or(peer, |(ip, _)| ip).to_string()),
            };
            dispatch(&request, shared)
        }
    };

    let _ = stream.write_all(&reply);
    let _ = stream.flush();
}

/// Validate, de-duplicate, and forward one request.
fn dispatch(request: &WebhookRequest, shared: &Shared) -> Vec<u8> {
    let now = unix_now();
    let validation = validate_request_at(&shared.config, request, now);
    if !validation.is_valid() {
        // Debug form server-side only; the response carries a fixed code.
        eprintln!("webhook: rejected {:?}", validation);
        return webhook_http::response(validation.status(), validation.code());
    }

    // Idempotency. Because `t` is inside the signed payload, the v1 digest is
    // unique per send, so it doubles as the delivery id — the sender needs no
    // extra header to specify or get wrong. A duplicate is answered 200, never
    // 4xx: a retrying sender did nothing wrong, and a 4xx would make it retry
    // harder.
    let event_id = delivery_id(request);
    if let Some(id) = &event_id {
        match shared.replay.lock() {
            Ok(mut guard) => {
                if !guard.admit(id, now) {
                    return webhook_http::response(200, "duplicate_ignored");
                }
            }
            Err(_) => return webhook_http::response(500, "internal_error"),
        }
    }

    match webhook_source::request_to_event(
        request,
        shared.config.machine.as_deref(),
        event_id.as_deref(),
    ) {
        Ok(event) => {
            // A dropped receiver used to be acknowledged as success: the old code
            // did `let _ = sender.send(event)` then returned 200, so a sender was
            // told its delivery was accepted when nothing would ever process it.
            if shared.sender.send(event).is_err() {
                return webhook_http::response(503, "event_pipeline_closed");
            }
            webhook_http::response(200, "accepted")
        }
        Err(e) => {
            eprintln!("webhook: body rejected: {e}");
            webhook_http::response(400, "invalid_body")
        }
    }
}

/// Delivery id: the first `v1` digest, which is unique per send.
fn delivery_id(request: &WebhookRequest) -> Option<String> {
    let raw = request.headers.get(SIG_HEADER)?;
    crate::core::webhook_sig::parse_forjar_signature(raw)
        .v1
        .into_iter()
        .next()
}

/// Validation outcome for a request, exposed for the CLI's dry-run path.
#[must_use]
pub fn validate_for_display(config: &WebhookConfig, request: &WebhookRequest) -> ValidationResult {
    validate_request_at(config, request, unix_now())
}
