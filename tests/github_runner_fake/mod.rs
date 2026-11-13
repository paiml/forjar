//! forjar#1000 (phase 2): a local fake of the slice of the GitHub REST API the
//! `github_runner` resource talks to, plus the rig that wires a real `forjar`
//! binary to it. No network, no real App: the fake lives in the test process
//! on `127.0.0.1:0`, records every request, and answers from scripted state.
//!
//! Nothing here is a real credential. The RSA key is generated at test time with
//! `openssl genrsa`; the installation and registration tokens are fixed marker
//! strings so a test can search for them.

#![allow(dead_code)]

mod files;
mod listing;
use files::{generate_key, make_shim, make_tarball};
pub use listing::Listing;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

/// What the fake hands out as the installation access token.
pub const INSTALL_TOKEN: &str = "ghs_FAKEINSTALLATIONTOKENq1w2e3r4";
/// What the fake hands out as the 1 h registration token.
pub const REG_TOKEN: &str = "AFAKEREGISTRATIONTOKENz9y8x7w6";
/// What the fake hands out as the 1 h remove token.
pub const REMOVE_TOKEN: &str = "AFAKEREMOVETOKENk5j4h3g2";

/// A seat the fake lists. `custom_labels` are the type-`custom` labels; the
/// fake adds GitHub's implicit `self-hosted`, `Linux` and `X64` itself.
#[derive(Clone)]
pub struct ListedRunner {
    pub name: String,
    pub custom_labels: Vec<String>,
    pub group: String,
}

impl ListedRunner {
    pub fn new(custom_labels: &[&str], group: &str) -> Self {
        Self {
            name: SEAT_NAME.to_string(),
            custom_labels: custom_labels.iter().map(|s| (*s).to_string()).collect(),
            group: group.to_string(),
        }
    }
}

/// One request the fake saw.
#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub bearer: String,
}

#[derive(Default)]
pub struct State {
    pub runners: Vec<ListedRunner>,
    pub requests: Vec<Request>,
    /// Listed after the first registration-token request is served: the fake's
    /// model of "the seat registered itself".
    pub appear_on_token: Option<ListedRunner>,
    /// When true, the first remove-token request unlists every runner: the
    /// fake's model of "the seat ran `config.sh remove`".
    pub vanish_on_remove_token: bool,
    pub listing: Listing,
}

pub struct FakeGithub {
    pub url: String,
    pub state: Arc<Mutex<State>>,
}

impl FakeGithub {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake");
        let url = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(State::default()));
        let shared = Arc::clone(&state);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                serve(stream, &shared);
            }
        });
        Self { url, state }
    }

    pub fn list(&self, runner: ListedRunner) {
        self.state.lock().unwrap().runners.push(runner);
    }

    pub fn appear_on_token(&self, runner: ListedRunner) {
        self.state.lock().unwrap().appear_on_token = Some(runner);
    }

    pub fn vanish_on_remove_token(&self) {
        self.state.lock().unwrap().vanish_on_remove_token = true;
    }

    pub fn requests(&self) -> Vec<Request> {
        self.state.lock().unwrap().requests.clone()
    }

    /// How many requests hit a path ending in `suffix` with `method`.
    pub fn count(&self, method: &str, suffix: &str) -> usize {
        self.requests()
            .iter()
            .filter(|r| r.method == method && r.path.ends_with(suffix))
            .count()
    }

    pub fn remove_token_requests(&self) -> usize {
        self.count("POST", "/actions/runners/remove-token")
    }

    pub fn token_requests(&self) -> usize {
        self.count("POST", "/actions/runners/registration-token")
    }

    /// The bearer the fake saw on the `access_tokens` request: the App JWT.
    pub fn jwt_seen(&self) -> String {
        self.requests()
            .iter()
            .find(|r| r.path.ends_with("/access_tokens"))
            .map(|r| r.bearer.clone())
            .unwrap_or_default()
    }
}

fn serve(stream: TcpStream, state: &Arc<Mutex<State>>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("").to_string();
    let (mut bearer, mut length) = (String::new(), 0usize);
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).unwrap_or(0) == 0 || header.trim().is_empty() {
            break;
        }
        let lower = header.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            length = v.trim().parse().unwrap_or(0);
        }
        if lower.starts_with("authorization:") {
            bearer = header["authorization:".len()..]
                .trim()
                .trim_start_matches("Bearer ")
                .trim_start_matches("token ")
                .to_string();
        }
    }
    let mut body = vec![0u8; length];
    let _ = reader.read_exact(&mut body);
    let path = target.split('?').next().unwrap_or("").to_string();
    let page_two = target.contains("page=2");
    let (status, json) = {
        let mut st = state.lock().unwrap();
        st.requests.push(Request {
            method: method.clone(),
            path: path.clone(),
            bearer: bearer.clone(),
        });
        route(&mut st, &method, &path, &bearer, page_two)
    };
    let reply = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
        json.len()
    );
    let mut out = stream;
    let _ = out.write_all(reply.as_bytes());
}

fn runner_json(r: &ListedRunner) -> serde_json::Value {
    let mut labels = vec![
        serde_json::json!({"id": 1, "name": "self-hosted", "type": "read-only"}),
        serde_json::json!({"id": 2, "name": "Linux", "type": "read-only"}),
        serde_json::json!({"id": 3, "name": "X64", "type": "read-only"}),
    ];
    for (i, l) in r.custom_labels.iter().enumerate() {
        labels.push(serde_json::json!({"id": 10 + i, "name": l, "type": "custom"}));
    }
    serde_json::json!({"id": 99, "name": r.name, "os": "linux", "status": "online", "busy": false, "labels": labels})
}

fn route(st: &mut State, method: &str, path: &str, bearer: &str, page_two: bool) -> (u16, String) {
    let is_jwt = bearer.matches('.').count() == 2;
    let app_scope = path.starts_with("/app/") || path.ends_with("/installation");
    if app_scope && !is_jwt {
        return (
            401,
            r#"{"message":"A JSON web token could not be decoded"}"#.into(),
        );
    }
    if !app_scope && bearer != INSTALL_TOKEN {
        return (401, r#"{"message":"Bad credentials"}"#.into());
    }
    let body = match (method, path) {
        ("GET", p) if p.ends_with("/installation") => serde_json::json!({"id": 7890123}),
        ("POST", p) if p.ends_with("/access_tokens") => {
            serde_json::json!({"token": INSTALL_TOKEN, "expires_at": "2099-01-01T00:00:00Z"})
        }
        ("POST" | "DELETE", p) if p.contains("/actions/runners/") => {
            return token_or_delete(st, method, p);
        }
        ("GET", p) if p.ends_with("/actions/runner-groups") => {
            let groups: Vec<_> = group_names(st)
                .iter()
                .enumerate()
                .map(|(i, n)| serde_json::json!({"id": i + 1, "name": n}))
                .collect();
            serde_json::json!({"total_count": groups.len(), "runner_groups": groups})
        }
        ("GET", p) if p.contains("/actions/runner-groups/") && p.ends_with("/runners") => {
            let id: usize = p
                .split("/runner-groups/")
                .nth(1)
                .and_then(|s| s.split('/').next())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let name = group_names(st)
                .get(id.wrapping_sub(1))
                .cloned()
                .unwrap_or_default();
            let r: Vec<_> = st.runners.iter().filter(|r| r.group == name).collect();
            runners_page(&r, page_two)
        }
        ("GET", p) if p.ends_with("/actions/runners") => listing::body(st, page_two),
        _ => return (404, r#"{"message":"Not Found"}"#.into()),
    };
    (if method == "POST" { 201 } else { 200 }, body.to_string())
}

/// The registration-token, remove-token and delete-runner endpoints.
fn token_or_delete(st: &mut State, method: &str, path: &str) -> (u16, String) {
    let token = |t: &str| serde_json::json!({"token": t, "expires_at": "2099-01-01T00:00:00Z"});
    if method == "DELETE" {
        st.runners.clear();
        return (204, String::new());
    }
    let body = if path.ends_with("/registration-token") {
        if let Some(r) = st.appear_on_token.take() {
            st.runners.push(r);
        }
        token(REG_TOKEN)
    } else if path.ends_with("/remove-token") {
        if std::mem::take(&mut st.vanish_on_remove_token) {
            st.runners.clear();
        }
        token(REMOVE_TOKEN)
    } else {
        return (404, r#"{"message":"Not Found"}"#.into());
    };
    (201, body.to_string())
}

fn runners_page(runners: &[&ListedRunner], page_two: bool) -> serde_json::Value {
    let shown: Vec<_> = if page_two {
        vec![]
    } else {
        runners.iter().map(|r| runner_json(r)).collect()
    };
    serde_json::json!({"total_count": runners.len(), "runners": shown})
}

/// The groups the fake knows: `Default` first, then every group a listed seat is in.
fn group_names(st: &State) -> Vec<String> {
    let mut names = vec!["Default".to_string()];
    for r in &st.runners {
        if !names.contains(&r.group) {
            names.push(r.group.clone());
        }
    }
    names
}

/// The merged output of one `forjar` run.
pub struct Out {
    pub text: String,
    pub stdout: String,
    pub stderr: String,
    pub ok: bool,
}

/// A tempdir wired to a fake GitHub, a fake runner tarball, a manifest, and a
/// `bash` shim that files every script the local transport hands to the "seat".
pub struct Rig {
    pub dir: tempfile::TempDir,
    pub fake: FakeGithub,
    pub cfg: PathBuf,
    pub key_pem: String,
    pub tarball_sha: String,
    path_env: String,
}

pub const SEAT_NAME: &str = "host-gate-1";

impl Rig {
    /// `labels` is the YAML flow list the manifest declares; `with_installation`
    /// writes `installation_id` into the manifest or leaves it to be looked up.
    pub fn new(labels: &str, with_installation: bool) -> Self {
        Self::build(labels, with_installation, None)
    }

    /// Rewrite the manifest so the seat is `state: absent`.
    pub fn set_absent(&self) {
        let text = std::fs::read_to_string(&self.cfg).unwrap();
        let text = text.replace(
            "    type: github_runner\n",
            "    type: github_runner\n    state: absent\n",
        );
        std::fs::write(&self.cfg, text).unwrap();
    }

    /// A repo-scoped seat in `example-org/<repo>` (no runner group).
    pub fn new_repo(labels: &str, repo: &str) -> Self {
        Self::build(labels, true, Some(repo))
    }

    fn build(labels: &str, with_installation: bool, repo: Option<&str>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let key_pem = generate_key(root);
        let secrets = root.join("secrets");
        std::fs::create_dir_all(&secrets).unwrap();
        std::fs::write(secrets.join("runner-app-pem"), &key_pem).unwrap();
        let tarball_sha = make_tarball(root);
        make_shim(root);
        let real_path = std::env::var("PATH").unwrap_or_default();
        let path_env = format!("{}:{real_path}", root.join("shim").display());
        let user =
            String::from_utf8(Command::new("id").arg("-un").output().unwrap().stdout).unwrap();
        let install = if with_installation {
            "      installation_id: 7890123\n"
        } else {
            ""
        };
        let scope_lines = match repo {
            Some(r) => format!("    scope: repo\n    repo: {r}\n"),
            None => "    scope: org\n    group: default\n".to_string(),
        };
        let cfg = root.join("forjar.yaml");
        std::fs::write(
            &cfg,
            format!(
                "version: '1.0'\nname: github-runner-seat\n\
                 secrets:\n  provider: file\n  path: \"{secrets}\"\n\
                 machines:\n  local:\n    hostname: localhost\n    addr: localhost\n    transport: local\n\
                 resources:\n  gate-seat-1:\n    type: github_runner\n    machine: local\n{scope_lines}\
                 \x20   owner: example-org\n    name: {SEAT_NAME}\n    labels: {labels}\n\
                 \x20   version: 2.330.0\n    sha256: {tarball_sha}\n    work_dir: {work}\n    user: {user}\n\
                 \x20   app:\n      app_id: 123456\n{install}      private_key: \"{{{{secrets.runner-app-pem}}}}\"\n",
                secrets = secrets.display(),
                work = root.join("seat/gate-1").display(),
                user = user.trim(),
            ),
        )
        .unwrap();
        Self {
            dir,
            fake: FakeGithub::start(),
            cfg,
            key_pem,
            tarball_sha,
            path_env,
        }
    }

    pub fn run(&self, state: &str, verb: &[&str]) -> Out {
        let root = self.dir.path();
        let mut args: Vec<String> = verb.iter().map(|s| (*s).to_string()).collect();
        args.extend([
            "-f".into(),
            self.cfg.display().to_string(),
            "--state-dir".into(),
            state.into(),
        ]);
        let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
            .current_dir(root)
            .env("PATH", &self.path_env)
            .env("FORJAR_GITHUB_API_URL", &self.fake.url)
            .env(
                "FORJAR_GITHUB_RUNNER_DOWNLOAD_BASE",
                format!("file://{}/dl", root.display()),
            )
            .args(&args)
            .output()
            .expect("forjar must run");
        let (stdout, stderr) = (
            String::from_utf8_lossy(&out.stdout).to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        );
        Out {
            text: format!("{stdout}{stderr}"),
            stdout,
            stderr,
            ok: out.status.success(),
        }
    }

    pub fn apply(&self, state: &str) -> Out {
        self.run(state, &["apply", "--yes"])
    }

    pub fn check(&self, state: &str) -> Out {
        self.run(state, &["check"])
    }

    /// Number of scripts the local transport handed to `bash` on the "seat".
    pub fn seat_commands(&self) -> usize {
        self.seat_scripts().len()
    }

    /// Every script the seat received on stdin, in order, except forjar's own
    /// disk-inode preflight (`df -i -P -T`), which every apply runs for every
    /// resource type and which is not the resource doing anything on the seat.
    pub fn seat_scripts(&self) -> Vec<String> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(self.dir.path().join("seatlog"))
            .map(|d| d.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        files.retain(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("script."))
        });
        files.sort();
        files
            .iter()
            .map(|p| std::fs::read_to_string(p).unwrap_or_default())
            .filter(|s| !s.contains("df -i -P -T"))
            .collect()
    }

    /// Lines the fake `config.sh` logged: its argv, one call per line.
    pub fn config_calls(&self) -> Vec<String> {
        read_lines(&self.dir.path().join("seatlog/config-calls"))
    }

    /// What the fake `config.sh` found in `ACTIONS_RUNNER_INPUT_TOKEN`.
    pub fn token_seen_by_config(&self) -> String {
        std::fs::read_to_string(self.dir.path().join("seatlog/token-seen")).unwrap_or_default()
    }

    /// Change exactly one byte of the served tarball.
    pub fn corrupt_tarball(&self) {
        let p = self
            .dir
            .path()
            .join("dl/v2.330.0/actions-runner-linux-x64-2.330.0.tar.gz");
        let mut bytes = std::fs::read(&p).unwrap();
        let mid = bytes.len() / 2;
        bytes[mid] ^= 0x01;
        std::fs::write(&p, bytes).unwrap();
    }

    pub fn runner_dir(&self) -> PathBuf {
        self.dir.path().join("seat/gate-1")
    }

    pub fn state_dir(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
}

fn read_lines(p: &Path) -> Vec<String> {
    std::fs::read_to_string(p)
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// Every file under `dir`, concatenated with its path: the text a secret must
/// be absent from.
pub fn tree_text(dir: &Path) -> String {
    let mut out = String::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push_str(&format!("--- {}\n", p.display()));
                out.push_str(&String::from_utf8_lossy(
                    &std::fs::read(&p).unwrap_or_default(),
                ));
            }
        }
    }
    out
}
