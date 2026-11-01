//! FJ-030: Docker container resource handler.
//!
//! Manages Docker containers as resources: pull, run, stop, remove.
//! This is distinct from container *transport* (FJ-021) — this manages
//! containers deployed ON machines, not containers used AS machines.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::verdict;

/// IMG-001: the `@sha256:<64 hex>` digest of an image reference, if it has a well-formed one.
pub fn image_digest(image: &str) -> Option<&str> {
    let d = image.rsplit_once("@")?.1;
    let hex = d.strip_prefix("sha256:")?;
    (hex.len() == 64
        && hex
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
    .then_some(d)
}

/// Shell fragment: succeeds iff the local image carries `digest` in its RepoDigests.
fn repodigest_check(image: &str) -> String {
    let digest = image_digest(image).unwrap_or("MALFORMED");
    format!(
        "docker image inspect --format {} {} 2>/dev/null | grep -qF {}",
        sh_squote("{{range .RepoDigests}}{{.}}{{\"\\n\"}}{{end}}"),
        sh_squote(image),
        sh_squote(&format!("@{digest}"))
    )
}

/// Generate shell script to check if a container is running.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    if resource.state.as_deref() == Some("pulled") {
        let image = resource.image.as_deref().unwrap_or("unknown");
        return verdict::single(
            &repodigest_check(image),
            &format!("pulled:{image}"),
            &format!("not-pulled:{image}"),
        );
    }
    let n = sh_squote(name);
    verdict::single(
        &format!("docker inspect -f '{{{{.State.Running}}}}' {n} >/dev/null 2>&1"),
        &format!("exists:{name}"),
        &format!("missing:{name}"),
    )
}

/// Generate shell script to manage a container.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let state = resource.state.as_deref().unwrap_or("running");
    let image = resource.image.as_deref().unwrap_or("unknown");
    let n = sh_squote(name);

    match state {
        // IMG-001: pull by digest and verify; never runs a container.
        "pulled" => format!(
            "set -euo pipefail\ndocker pull {}\n{} || {{ echo {} >&2; exit 1; }}",
            sh_squote(image),
            repodigest_check(image),
            sh_squote(&format!(
                "pulled image lacks the declared RepoDigest: {image}"
            ))
        ),
        "absent" => format!(
            "set -euo pipefail\n\
             docker stop {n} 2>/dev/null || true\n\
             docker rm {n} 2>/dev/null || true"
        ),
        "stopped" => format!(
            "set -euo pipefail\n\
             docker stop {n} 2>/dev/null || true"
        ),
        _ => {
            // "running" or "present"
            let mut lines = vec![
                "set -euo pipefail".to_string(),
                format!("docker pull {}", sh_squote(image)),
            ];

            // Stop and remove existing container if it exists
            lines.push(format!("docker stop {n} 2>/dev/null || true"));
            lines.push(format!("docker rm {n} 2>/dev/null || true"));

            // Build run command
            let mut run_args = vec!["docker run -d".to_string()];
            run_args.push(format!("--name {n}"));

            if let Some(ref restart) = resource.restart {
                run_args.push(format!("--restart {}", sh_squote(restart)));
            }

            for port in &resource.ports {
                run_args.push(format!("-p {}", sh_squote(port)));
            }

            for env in &resource.environment {
                run_args.push(format!("-e {}", sh_squote(env)));
            }

            for vol in &resource.volumes {
                run_args.push(format!("-v {}", sh_squote(vol)));
            }

            run_args.push(sh_squote(image));

            // Append command if specified (intentionally arbitrary shell).
            if let Some(ref cmd) = resource.command {
                run_args.push(cmd.clone());
            }

            lines.push(run_args.join(" \\\n  "));

            lines.join("\n")
        }
    }
}

/// Generate shell to query container state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    if resource.state.as_deref() == Some("pulled") {
        let image = resource.image.as_deref().unwrap_or("unknown");
        return format!(
            "docker image inspect --format {} {} 2>/dev/null || echo {}",
            sh_squote("{{range .RepoDigests}}{{.}},{{end}}"),
            sh_squote(image),
            sh_squote(&format!("image=MISSING:{image}"))
        );
    }
    let n = sh_squote(name);
    // DECLARED INTENT ONLY — NOT `docker inspect`'s full output.
    //
    // This hashed the ENTIRE inspect document, which contains State.StartedAt,
    // State.FinishedAt and State.Status. Measured on intel, the same container
    // twice, 12 seconds apart:
    //
    //     inspect sha t0: 52eea127b4b425dd
    //     inspect sha t1: c1e3029d2739ffa3
    //
    // So a docker resource's live_hash was PERMANENTLY volatile — always
    // "drifted". That was harmless while only `forjar drift` read it. #307 made
    // the apply gate read it too, and forjar's docker apply is
    // `docker stop; docker rm; docker run` — so EVERY apply tore down and
    // recreated every container. For machines/intel/forjar.yaml's `ci-registry`
    // that is paiml/infra#285, the 2026-08-24 fleet-wide CI outage, converted
    // into a guaranteed recurrence. (forjar#310.)
    //
    // A container's identity here is what the resource DECLARES: image, ports,
    // mounts, restart policy, env — plus whether it is running at all. How long
    // it has been up is not drift. Same principle as the directory-`size`
    // exclusion in file.rs: an observable that moves on its own is not evidence
    // of divergence from intent.
    let fmt = "{{.Config.Image}}|{{.Image}}|{{.HostConfig.RestartPolicy.Name}}|{{range $p,$c := .HostConfig.PortBindings}}{{$p}}->{{range $c}}{{.HostPort}}{{end}},{{end}}|{{range .Mounts}}{{.Type}}:{{.Name}}:{{.Source}}:{{.Destination}},{{end}}|{{range .Config.Env}}{{.}},{{end}}|running={{.State.Running}}";
    format!(
        "docker inspect --format {} {n} 2>/dev/null && echo {} || echo {}",
        sh_squote(fmt),
        sh_squote(&format!("container={name}")),
        sh_squote(&format!("container=MISSING:{name}"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn docker_resource(name: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Docker,
            machine: MachineTarget::Single("m1".to_string()),
            name: Some(name.to_string()),
            image: Some("nginx:latest".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn fj154_docker_name_quote_neutralized() {
        let r = docker_resource("c';reboot;'");
        let script = apply_script(&r);
        assert!(script.contains("'\"'\"'"), "{script}");
        assert!(!script.contains("docker stop 'c';reboot"), "{script}");
    }

    #[test]
    fn fj154_docker_image_and_run_fields_quoted() {
        let mut r = docker_resource("web");
        r.image = Some("img';id;'".to_string());
        r.ports = vec!["8080:80".to_string()];
        r.environment = vec!["KEY=v';id;'".to_string()];
        r.volumes = vec!["/data:/data".to_string()];
        let script = apply_script(&r);
        assert!(script.contains("'\"'\"'"), "{script}");
        assert!(script.contains("-p '8080:80'"), "{script}");
        assert!(script.contains("-v '/data:/data'"), "{script}");
    }

    #[test]
    fn fj154_docker_benign_unchanged() {
        let r = docker_resource("web");
        let script = apply_script(&r);
        assert!(script.contains("docker pull 'nginx:latest'"));
        assert!(script.contains("--name 'web'"));
        assert!(check_script(&r).contains("exists:web"));
        assert!(state_query_script(&r).contains("container=MISSING:web"));
    }
}

#[cfg(test)]
mod tests_pulled {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn pulled(image: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Docker,
            machine: MachineTarget::Single("m1".to_string()),
            name: Some("golden".to_string()),
            image: Some(image.to_string()),
            state: Some("pulled".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn pulled_pulls_by_digest_and_verifies_repodigests() {
        let r = pulled(&format!("ghcr.io/paiml/build@{DIGEST}"));
        let s = apply_script(&r);
        assert!(
            s.contains(&format!("docker pull 'ghcr.io/paiml/build@{DIGEST}'")),
            "{s}"
        );
        assert!(s.contains("docker image inspect"), "{s}");
        assert!(s.contains("RepoDigests"), "{s}");
        assert!(s.contains(DIGEST), "{s}");
    }

    #[test]
    fn pulled_never_runs_a_container() {
        let s = apply_script(&pulled(&format!("img@{DIGEST}")));
        for banned in ["docker run", "docker stop", "docker rm"] {
            assert!(!s.contains(banned), "{banned} in {s}");
        }
    }

    #[test]
    fn pulled_check_and_query_ask_the_image_not_a_container() {
        let r = pulled(&format!("img@{DIGEST}"));
        let c = check_script(&r);
        assert!(
            c.contains("docker image inspect") && c.contains(DIGEST),
            "{c}"
        );
        assert!(!c.contains("State.Running"), "{c}");
        let q = state_query_script(&r);
        assert!(
            q.contains("RepoDigests") && !q.contains("State.Running"),
            "{q}"
        );
    }

    #[test]
    fn pulled_quotes_hostile_image() {
        let s = apply_script(&pulled(&format!("i';id;'@{DIGEST}")));
        assert!(s.contains("'\"'\"'"), "{s}");
    }
}
