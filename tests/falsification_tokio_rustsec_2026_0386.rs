//! RUSTSEC-2026-0386 (PMAT-756): tokio's safe `AsyncFd::new` family violates I/O safety.
//! Patched lines: `>= 1.51.6, < 1.52.0` and `>= 1.53.3`. `ci / lint` runs cargo-deny on the
//! lock, so the lock must not pin an unpatched tokio. No deny.toml ignore stands in for this.
use std::path::PathBuf;

fn locked_tokio() -> (u32, u32, u32) {
    let lock = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    let text = std::fs::read_to_string(lock).expect("Cargo.lock is readable");
    let mut lines = text.lines();
    while let Some(l) = lines.next() {
        if l == "name = \"tokio\"" {
            let v = lines.next().expect("version line follows name");
            let v = v.trim_start_matches("version = \"").trim_end_matches('"');
            let p: Vec<u32> = v.split('.').map(|n| n.parse().expect("numeric")).collect();
            return (p[0], p[1], p[2]);
        }
    }
    panic!("Cargo.lock has no tokio entry");
}

#[test]
fn locked_tokio_is_a_patched_version() {
    let v = locked_tokio();
    let patched = (v >= (1, 51, 6) && v < (1, 52, 0)) || v >= (1, 53, 3);
    assert!(
        patched,
        "RUSTSEC-2026-0386: Cargo.lock pins tokio {v:?}, unpatched"
    );
}
