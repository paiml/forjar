//! forjar#1000: the `github_runner` resource — a persistent self-hosted GitHub
//! Actions runner seat, registered through a GitHub App.
//!
//! Contract: `contracts/github-runner-v1.yaml`.
//!
//! Everything that holds a credential happens HERE, on the control host: the
//! App key is read through the manifest `secrets:` provider, the JWT is signed
//! here, and the installation token stays in [`Session`]. The seat receives one
//! thing, the 1 h registration token, on stdin (see `script`). None of the
//! types in this module implements `Debug`, so a stray `{:?}` cannot print one.
//!
//! * [`check`] lists the scope's runners and returns a [`Verdict`].
//! * `state: absent` deregisters: one remove-token request, `config.sh remove`
//!   on the seat, and the listing deleted by API if it outlives the seat.
//! * [`apply`] adopts a matching seat (zero commands on it), refuses a drifted
//!   one (zero commands, zero registration-token requests, never `--replace`),
//!   and registers an absent one with exactly one registration-token request.

mod api;
mod jwt;
mod labels;
mod script;
pub(crate) use script::is_sha256_hex;

use crate::core::types::{Machine, Resource};
use crate::transport::{self, ExecOutput};
use api::Github;
use labels::ListedLabel;

/// What the listing says about the seat.
pub enum Verdict {
    /// No runner of that name in the scope.
    Absent,
    /// Listed with the declared labels and group.
    Equal,
    /// Listed with a different label set or group; the text names both.
    Drift(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Org,
    Repo,
}

/// A validated, fully resolved `github_runner` declaration.
struct Seat {
    scope: Scope,
    owner: String,
    repo: Option<String>,
    name: String,
    labels: Vec<String>,
    group: Option<String>,
    version: String,
    sha256: String,
    work_dir: String,
    user: String,
    app_id: u64,
    installation_id: Option<u64>,
    private_key: String,
}

fn need(value: &Option<String>, field: &str, name: &str) -> Result<String, String> {
    match value.as_deref().map(str::trim) {
        Some(v) if !v.is_empty() => Ok(v.to_string()),
        _ => Err(format!("github_runner `{name}`: `{field}` is required")),
    }
}

/// A repo the scope does not use is refused, never dropped: an org seat
/// carrying a `repo` would register across every repo in the org.
fn seat_repo(r: &Resource, scope: Scope, name: &str) -> Result<Option<String>, String> {
    match scope {
        Scope::Repo => need(&r.repo, "repo", name).map(Some),
        Scope::Org if r.repo.as_deref().is_some_and(|s| !s.trim().is_empty()) => Err(format!(
            "github_runner `{name}`: `repo` must not be set when `scope` is `org`"
        )),
        Scope::Org => Ok(None),
    }
}

/// Anything but `absent` is applied as present, so an unknown `state` (`removed`,
/// `Absent`) is refused rather than registering a runner.
fn check_state(r: &Resource, name: &str) -> Result<(), String> {
    match r.state.as_deref() {
        None | Some("present" | "absent") => Ok(()),
        Some(other) => Err(format!(
            "github_runner `{name}`: `state` is {other:?}, expected `present` or `absent`"
        )),
    }
}

impl Seat {
    fn from_resource(r: &Resource) -> Result<Self, String> {
        let name = need(&r.name, "name", "?")?;
        let spec = &r.runner;
        let app = spec
            .app
            .as_ref()
            .ok_or_else(|| format!("github_runner `{name}`: `app` is required"))?;
        let private_key = need(&app.private_key, "app.private_key", &name)?;
        if private_key.contains("{{secrets.") {
            return Err(format!(
                "github_runner `{name}`: app.private_key was not supplied by the secrets provider"
            ));
        }
        let scope = match spec.scope.as_deref() {
            Some("org") => Scope::Org,
            Some("repo") => Scope::Repo,
            _ => {
                return Err(format!(
                    "github_runner `{name}`: `scope` must be org or repo"
                ))
            }
        };
        let repo = seat_repo(r, scope, &name)?;
        check_state(r, &name)?;
        let work_dir = spec
            .work_dir
            .clone()
            .unwrap_or_else(|| format!("/opt/github-runner/{name}"));
        Ok(Self {
            scope,
            owner: need(&r.owner, "owner", &name)?,
            repo,
            labels: spec.labels.clone(),
            group: r.group.clone().filter(|_| scope == Scope::Org),
            version: need(&r.version, "version", &name)?,
            sha256: {
                let s = need(&spec.sha256, "sha256", &name)?.to_ascii_lowercase();
                if !script::is_sha256_hex(&s) {
                    return Err(format!(
                        "github_runner `{name}`: `sha256` must be exactly 64 hex characters"
                    ));
                }
                s
            },
            work_dir,
            user: need(&spec.user, "user", &name)?,
            app_id: app
                .app_id
                .ok_or_else(|| format!("github_runner `{name}`: `app.app_id` is required"))?,
            installation_id: app.installation_id,
            private_key,
            name,
        })
    }

    /// `/orgs/<owner>` or `/repos/<owner>/<repo>`.
    fn base(&self) -> Result<String, String> {
        match (self.scope, &self.repo) {
            (Scope::Org, _) => Ok(format!("/orgs/{}", self.owner)),
            (Scope::Repo, Some(repo)) => Ok(format!("/repos/{}/{repo}", self.owner)),
            (Scope::Repo, None) => {
                Err(format!("github_runner `{}`: `repo` is required", self.name))
            }
        }
    }

    fn registration_url(&self) -> String {
        match &self.repo {
            Some(repo) => format!("https://github.com/{}/{repo}", self.owner),
            None => format!("https://github.com/{}", self.owner),
        }
    }

    fn plan(&self) -> script::SeatPlan {
        let arm64 = self.labels.iter().any(|l| l.eq_ignore_ascii_case("arm64"));
        script::SeatPlan {
            tarball_url: script::tarball_url(&self.version, arm64),
            sha256: self.sha256.to_ascii_lowercase(),
            install_dir: format!("{}/runner", self.work_dir),
            work_dir: self.work_dir.clone(),
            user: self.user.clone(),
            registration_url: self.registration_url(),
            name: self.name.clone(),
            labels: labels::custom_for_config(&self.labels).join(","),
            group: self.group.clone(),
        }
    }
}

/// An authenticated view of one scope: the installation token lives here and
/// nowhere else.
struct Session<'a> {
    gh: Github,
    seat: &'a Seat,
    base: String,
    token: String,
}

struct Found {
    id: u64,
    labels: Vec<ListedLabel>,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn string_field(json: &serde_json::Value, key: &str, what: &str) -> Result<String, String> {
    json.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("github_runner: GitHub's {what} response had no `{key}`"))
}

impl<'a> Session<'a> {
    fn open(seat: &'a Seat) -> Result<Self, String> {
        let base = seat.base()?;
        let jwt = jwt::app_jwt(seat.app_id, &seat.private_key, now())?;
        let gh = Github::from_env();
        let installation = match seat.installation_id {
            Some(id) => id,
            None => gh
                .get(&format!("{base}/installation"), &jwt)?
                .get("id")
                .and_then(serde_json::Value::as_u64)
                .ok_or("github_runner: the App has no installation on this owner")?,
        };
        let granted = gh.post(
            &format!("/app/installations/{installation}/access_tokens"),
            &jwt,
        )?;
        let token = string_field(&granted, "token", "installation token")?;
        Ok(Self {
            gh,
            seat,
            base,
            token,
        })
    }

    fn find(&self) -> Result<Option<Found>, String> {
        for page in 1..=50 {
            let path = format!("{}/actions/runners?per_page=100&page={page}", self.base);
            let body = self.gh.get(&path, &self.token)?;
            // A listing without `runners` is unmeasured, never "absent".
            let runners = body
                .get("runners")
                .and_then(|r| r.as_array())
                .ok_or_else(|| {
                    format!(
                        "github_runner `{}`: the runner listing has no `runners` array",
                        self.seat.name
                    )
                })?;
            if let Some(r) = runners
                .iter()
                .find(|r| r["name"] == self.seat.name.as_str())
            {
                return Ok(Some(found_from(r)));
            }
            if runners.len() < 100 {
                return Ok(None);
            }
        }
        Err(format!(
            "github_runner `{}`: the runner listing has more than 50 pages; \
             cannot tell whether the seat exists",
            self.seat.name
        ))
    }

    /// The runner group a listed runner is in (org scope only).
    fn group_of(&self, runner_id: u64) -> Result<String, String> {
        let listing = format!("{}/actions/runner-groups?per_page=100", self.base);
        let groups = self.gh.get(&listing, &self.token)?;
        for g in groups["runner_groups"].as_array().into_iter().flatten() {
            let gid = g["id"].as_u64().unwrap_or(0);
            let members_path = format!(
                "{}/actions/runner-groups/{gid}/runners?per_page=100",
                self.base
            );
            let members = self.gh.get(&members_path, &self.token)?;
            if members["runners"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|m| m["id"].as_u64() == Some(runner_id))
            {
                return Ok(g["name"].as_str().unwrap_or_default().to_string());
            }
        }
        Err(format!(
            "github_runner `{}`: cannot determine which runner group the seat is in",
            self.seat.name
        ))
    }

    fn verdict(&self) -> Result<Verdict, String> {
        let Some(found) = self.find()? else {
            return Ok(Verdict::Absent);
        };
        let listed_group = match (&self.seat.group, self.seat.scope) {
            (Some(_), Scope::Org) => Some(self.group_of(found.id)?),
            _ => None,
        };
        Ok(
            match labels::difference(
                &self.seat.labels,
                &found.labels,
                self.seat.group.as_deref(),
                listed_group.as_deref(),
            ) {
                None => Verdict::Equal,
                Some(d) => Verdict::Drift(d),
            },
        )
    }

    /// The one request that mints the 1 h remove token (deregistration).
    fn remove_token(&self) -> Result<String, String> {
        let path = format!("{}/actions/runners/remove-token", self.base);
        let token = string_field(&self.gh.post(&path, &self.token)?, "token", "remove token")?;
        script::checked_token(token, "remove token")
    }

    /// Drop a listing GitHub still holds after the seat is gone.
    fn delete_listing(&self, runner_id: u64) -> Result<(), String> {
        let path = format!("{}/actions/runners/{runner_id}", self.base);
        self.gh.delete(&path, &self.token).map(|_| ())
    }

    /// The one request that mints the 1 h registration token.
    fn registration_token(&self) -> Result<String, String> {
        let path = format!("{}/actions/runners/registration-token", self.base);
        let token = string_field(
            &self.gh.post(&path, &self.token)?,
            "token",
            "registration token",
        )?;
        script::checked_token(token, "registration token")
    }
}

fn found_from(runner: &serde_json::Value) -> Found {
    let labels = runner["labels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| {
            Some(ListedLabel {
                name: l["name"].as_str()?.to_string(),
                read_only: l["type"] == "read-only",
            })
        })
        .collect();
    Found {
        id: runner["id"].as_u64().unwrap_or(0),
        labels,
    }
}

/// `forjar check`: what GitHub lists for the seat, against what is declared.
pub fn check(resource: &Resource) -> Result<Verdict, String> {
    let seat = Seat::from_resource(resource)?;
    Session::open(&seat)?.verdict()
}

fn redact(text: &str, token: &str) -> String {
    text.replace(token, script::REDACTED)
}

fn adopted(name: &str) -> ExecOutput {
    ExecOutput {
        exit_code: 0,
        stdout: format!(
            "github_runner `{name}`: already registered with the declared labels and group; adopted, nothing run on the seat\n"
        ),
        stderr: String::new(),
    }
}

/// `forjar apply`: returns the outcome and the TRANSCRIPT form of the script
/// (the registration token replaced by a placeholder), which is what the run
/// log and `--trace` may show.
pub fn apply(
    machine: &Machine,
    resource: &Resource,
    timeout_secs: Option<u64>,
    ssh_retries: u32,
) -> (Result<ExecOutput, String>, String) {
    let seat = match Seat::from_resource(resource) {
        Ok(s) => s,
        Err(e) => return (Err(e), String::new()),
    };
    if resource.state.as_deref() == Some("absent") {
        return match deregister(machine, &seat, timeout_secs, ssh_retries) {
            Ok(done) => done,
            Err(e) => (Err(e), String::new()),
        };
    }
    match register_if_absent(machine, &seat, timeout_secs, ssh_retries) {
        Ok(done) => done,
        Err(e) => (Err(e), String::new()),
    }
}

/// `state: absent`. Not listed: nothing to do, zero seat commands, zero token
/// requests. Listed: one remove-token request and one seat script; if GitHub
/// still lists the runner afterwards (the seat was already wiped), the listing
/// is deleted by id. A failing seat script is returned as a failure and the
/// listing is left alone.
fn deregister(
    machine: &Machine,
    seat: &Seat,
    timeout_secs: Option<u64>,
    ssh_retries: u32,
) -> Result<(Result<ExecOutput, String>, String), String> {
    let session = Session::open(seat)?;
    let Some(found) = session.find()? else {
        let out = ExecOutput {
            exit_code: 0,
            stdout: format!(
                "github_runner `{}`: not registered; nothing to deregister\n",
                seat.name
            ),
            stderr: String::new(),
        };
        return Ok((Ok(out), String::new()));
    };
    let token = session.remove_token()?;
    let plan = seat.plan();
    let real = script::remove_script(&plan, &token);
    let transcript = script::remove_script(&plan, script::REDACTED);
    let out = transport::exec_script_retry(machine, &real, timeout_secs, ssh_retries)
        .map(|o| ExecOutput {
            exit_code: o.exit_code,
            stdout: redact(&o.stdout, &token),
            stderr: redact(&o.stderr, &token),
        })
        .map_err(|e| redact(&e, &token));
    if matches!(&out, Ok(o) if o.exit_code == 0) && session.find()?.is_some() {
        session.delete_listing(found.id)?;
    }
    Ok((out, transcript))
}

fn register_if_absent(
    machine: &Machine,
    seat: &Seat,
    timeout_secs: Option<u64>,
    ssh_retries: u32,
) -> Result<(Result<ExecOutput, String>, String), String> {
    let session = Session::open(seat)?;
    match session.verdict()? {
        Verdict::Equal => Ok((Ok(adopted(&seat.name)), String::new())),
        Verdict::Drift(d) => Err(format!(
            "github_runner `{}`: refusing to re-register a drifted seat (never --replace): {d}",
            seat.name
        )),
        Verdict::Absent => {
            let token = session.registration_token()?;
            let plan = seat.plan();
            let real = script::seat_script(&plan, &token);
            let transcript = script::seat_script(&plan, script::REDACTED);
            let out = transport::exec_script_retry(machine, &real, timeout_secs, ssh_retries);
            let out = out
                .map(|o| ExecOutput {
                    exit_code: o.exit_code,
                    stdout: redact(&o.stdout, &token),
                    stderr: redact(&o.stderr, &token),
                })
                .map_err(|e| redact(&e, &token));
            Ok((out, transcript))
        }
    }
}

#[cfg(test)]
mod tests;
