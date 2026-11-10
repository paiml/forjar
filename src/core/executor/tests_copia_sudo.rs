//! #927: the copia file transfer runs in the resource's declared privilege context.
//!
//! The copia scripts (signature, full transfer, patch) bypass `apply_script`,
//! so `sudo: true` never reached them. These tests observe the scripts at the
//! call site, through the `copia_apply_file_via` seam, so reverting any one of
//! the three `in_ctx` calls goes red; and they EXECUTE the wrapped patch and
//! full-transfer scripts under a fake `sudo`/`id` on `PATH` (the
//! `falsification_sudo_transport_closefrom_emulated.rs` convention), with a
//! literal carrying the heredoc delimiters, a `$VAR` and a backtick, and compare
//! the bytes written with the `sudo: false` run.

use super::helpers::{copia_apply_file, copia_apply_file_via};
use super::test_fixtures::local_machine;
use crate::copia;
use crate::core::codegen::in_declared_privilege_context;
use crate::core::types::{Resource, ResourceType};
use crate::transport::ExecOutput;
use std::path::Path;
use std::process::{Command, Stdio};

const SUDO_LINE: &str = "sudo bash \"$forjar_sudo_script\"";

/// Bytes that would break a naive heredoc: both delimiters on their own line,
/// an expansion, a command substitution and a quote.
const HOSTILE: &[u8] = b"FORJAR_SUDO\n$HOME `id -u` $(echo x)\nFORJAR_B64\n'q' \"d\" \\\n";

fn copia_resource(sudo: bool, path: &str, source: &str) -> Resource {
    Resource {
        resource_type: ResourceType::File,
        path: Some(path.to_string()),
        source: Some(source.to_string()),
        sudo,
        ..Default::default()
    }
}

fn ok(stdout: &str) -> ExecOutput {
    ExecOutput {
        exit_code: 0,
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

/// The signature-script output a receiver holding `basis` prints.
fn signature_output(basis: &[u8]) -> String {
    let mut out = format!("SIZE:{}\n", basis.len());
    for (i, block) in basis.chunks(copia::BLOCK_SIZE).enumerate() {
        let weak = copia::weak_checksum(block);
        let strong = blake3::hash(block).to_hex();
        out.push_str(&format!("{i} {weak} {strong}\n"));
    }
    out
}

/// Run `copia_apply_file_via`, answering the signature phase with `sig`, and
/// return the result plus every script handed to the transport.
fn record(r: &Resource, sig: ExecOutput) -> (Result<ExecOutput, String>, Vec<String>) {
    let mut seen = Vec::new();
    let mut sig = Some(sig);
    let res = copia_apply_file_via(r, &mut |s| {
        seen.push(s.to_string());
        Ok(sig.take().unwrap_or_else(|| ok("")))
    });
    (res, seen)
}

#[test]
fn test_927_new_file_both_scripts_are_wrapped_at_the_call_site() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.bin");
    std::fs::write(&src, b"payload").unwrap();
    let src = src.to_str().unwrap();
    for sudo in [true, false] {
        let r = copia_resource(sudo, "/opt/new/x", src);
        let (res, seen) = record(&r, ok("NEW_FILE\n"));
        assert!(res.unwrap().success());
        let want = [
            copia::signature_script("/opt/new/x"),
            copia::full_transfer_script("/opt/new/x", src, None, None, None).unwrap(),
        ];
        assert_eq!(seen.len(), 2, "signature + full transfer");
        for (got, raw) in seen.iter().zip(want) {
            assert_eq!(got.contains(SUDO_LINE), sudo, "sudo={sudo}:\n{got}");
            assert_eq!(*got, in_declared_privilege_context(&r, raw));
        }
    }
}

#[test]
fn test_927_patch_script_is_wrapped_at_the_call_site() {
    let dir = tempfile::tempdir().unwrap();
    let basis = vec![b'a'; copia::BLOCK_SIZE * 2];
    let mut new_data = basis.clone();
    new_data.splice(10..10, HOSTILE.iter().copied());
    let src = dir.path().join("src.bin");
    std::fs::write(&src, &new_data).unwrap();
    let sig_out = signature_output(&basis);
    for sudo in [true, false] {
        let r = copia_resource(sudo, "/opt/x", src.to_str().unwrap());
        let (res, seen) = record(&r, ok(&sig_out));
        assert!(res.unwrap().success());
        assert_eq!(seen.len(), 2, "signature + patch");
        let sig = copia::parse_signature(&sig_out).unwrap().unwrap();
        let delta = copia::rolling_delta(&new_data, &sig);
        let expected = blake3::hash(&new_data).to_hex().to_string();
        let raw = copia::patch_script("/opt/x", &delta, &expected, None, None, None);
        assert_eq!(
            seen[1].contains(SUDO_LINE),
            sudo,
            "sudo={sudo}:\n{}",
            seen[1]
        );
        assert_eq!(seen[1], in_declared_privilege_context(&r, raw));
    }
}

#[test]
fn test_927_signature_failure_stops_before_any_write() {
    let r = copia_resource(true, "/opt/x", "/nonexistent");
    let failed = ExecOutput {
        exit_code: 1,
        stdout: String::new(),
        stderr: "sudo: a password is required".to_string(),
    };
    let (res, seen) = record(&r, failed);
    let err = res.unwrap_err();
    assert!(err.contains("copia signature failed"), "{err}");
    assert_eq!(seen.len(), 1);
}

#[test]
fn test_927_transport_error_and_unreadable_source_propagate() {
    let r = copia_resource(true, "/opt/x", "/nonexistent/forjar-927");
    let err = copia_apply_file_via(&r, &mut |_| Err("ssh down".to_string())).unwrap_err();
    assert_eq!(err, "ssh down");
    // New file: the full transfer reads the source.
    let (res, seen) = record(&r, ok("NEW_FILE\n"));
    assert!(res.unwrap_err().contains("/nonexistent/forjar-927"));
    assert_eq!(seen.len(), 1);
    // Existing file: the delta reads the source.
    let (res, seen) = record(&r, ok(&signature_output(b"basis")));
    assert!(res.unwrap_err().contains("copia read source"));
    assert_eq!(seen.len(), 1);
    // A malformed signature is an error, not a transfer.
    let (res, seen) = record(&r, ok("not-a-number 1 x\n"));
    assert!(res.unwrap_err().contains("bad index"));
    assert_eq!(seen.len(), 1);
}

/// The production entry point hands its scripts to the real transport: a
/// `sudo: false` new file lands through the local transport.
#[test]
fn test_927_copia_apply_file_runs_on_the_local_transport() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.bin");
    std::fs::write(&src, HOSTILE).unwrap();
    let dest = dir.path().join("sub").join("dest.bin");
    let r = copia_resource(false, dest.to_str().unwrap(), src.to_str().unwrap());
    let out = copia_apply_file(&local_machine(), &r, Some(60)).unwrap();
    assert!(out.success(), "{}", out.stderr);
    assert_eq!(std::fs::read(&dest).unwrap(), HOSTILE);
}

/// `sudo` that records that it ran, then runs the command; `id -u` answers
/// non-root so the wrapper takes the sudo branch even in a root container.
const FAKE_SUDO: &str = "#!/usr/bin/env bash\n: > \"$FORJAR_FAKE_SUDO_MARK\"\nexec \"$@\"\n";
const FAKE_ID: &str = "#!/usr/bin/env bash\nif [ \"$1\" = \"-u\" ]; then echo 1000; exit 0; fi\n\
                       for real in /usr/bin/id /bin/id; do [ -x \"$real\" ] && exec \"$real\" \"$@\"; done\n\
                       exit 127\n";

fn fake_bin(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    for (name, body) in [("sudo", FAKE_SUDO), ("id", FAKE_ID)] {
        let p = dir.join(name);
        std::fs::write(&p, body).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Feed `script` to `bash -s` on stdin, as the transports do, with the fakes
/// first on `PATH`. Returns (exit code, whether the fake sudo ran).
fn run(script: &str, root: &Path) -> (Option<i32>, bool) {
    use std::io::Write;
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    fake_bin(&bin);
    let mark = root.join("sudo-ran");
    let _ = std::fs::remove_file(&mark);
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut child = Command::new("bash")
        .arg("-s")
        .env("PATH", path)
        .env("TMPDIR", root)
        .env("FORJAR_FAKE_SUDO_MARK", &mark)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let code = child.wait().unwrap().code();
    (code, mark.exists())
}

/// Execute the write script (the second one) `copia_apply_file_via` hands the
/// transport, against a receiver holding `basis` (None = new file); return the
/// destination's bytes and whether sudo ran.
fn apply_for_real(sudo: bool, basis: Option<&[u8]>, new_data: &[u8]) -> (Vec<u8>, bool) {
    let root = tempfile::tempdir().unwrap();
    let dest = root.path().join("dest").join("file.bin");
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    let sig = match basis {
        Some(b) => {
            std::fs::write(&dest, b).unwrap();
            signature_output(b)
        }
        None => "NEW_FILE\n".to_string(),
    };
    let src = root.path().join("src.bin");
    std::fs::write(&src, new_data).unwrap();
    let r = copia_resource(sudo, dest.to_str().unwrap(), src.to_str().unwrap());
    let (res, seen) = record(&r, ok(&sig));
    res.unwrap();
    let (code, sudo_ran) = run(&seen[1], root.path());
    assert_eq!(code, Some(0), "sudo={sudo}:\n{}", seen[1]);
    let leftovers: Vec<_> = std::fs::read_dir(root.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("forjar-sudo."))
        .collect();
    assert!(
        leftovers.is_empty(),
        "wrapper temp file left: {leftovers:?}"
    );
    (std::fs::read(&dest).unwrap(), sudo_ran)
}

#[test]
fn test_927_wrapped_patch_with_hostile_literal_writes_the_same_bytes() {
    let basis = vec![b'a'; copia::BLOCK_SIZE * 3];
    let mut new_data = basis.clone();
    new_data.splice(
        copia::BLOCK_SIZE + 5..copia::BLOCK_SIZE + 5,
        HOSTILE.iter().copied(),
    );
    let (elevated, ran) = apply_for_real(true, Some(&basis), &new_data);
    assert!(ran, "sudo: true must cross the sudo boundary");
    let (plain, ran) = apply_for_real(false, Some(&basis), &new_data);
    assert!(!ran, "sudo: false must not call sudo");
    assert_eq!(elevated, new_data);
    assert_eq!(plain, new_data);
}

#[test]
fn test_927_wrapped_full_transfer_with_hostile_payload_writes_the_same_bytes() {
    let (elevated, ran) = apply_for_real(true, None, HOSTILE);
    assert!(ran, "sudo: true must cross the sudo boundary");
    let (plain, ran) = apply_for_real(false, None, HOSTILE);
    assert!(!ran);
    assert_eq!(elevated, HOSTILE);
    assert_eq!(plain, HOSTILE);
}
