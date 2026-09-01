//! State queries for BLAKE3 hashing, and the scatter/gather transfer scripts.

use crate::core::shell_escape::{render_command_inline, sh_squote};
use crate::core::types::Resource;
use crate::resources::verdict;

/// Generate shell to query task state (for BLAKE3 hashing).
///
/// Hashes output_artifacts if specified, otherwise reports command string.
pub fn state_query_script(resource: &Resource) -> String {
    if !resource.output_artifacts.is_empty() {
        let hash_cmds: Vec<String> = resource
            .output_artifacts
            .iter()
            .map(|a| {
                let q = sh_squote(a);
                format!(
                    "[ -f {q} ] && b3sum {q} 2>/dev/null || echo {}",
                    sh_squote(&format!("missing:{a}"))
                )
            })
            .collect();
        return hash_cmds.join("\n");
    }

    // A COMPLETION CHECK IS AN OBSERVABLE. AN ECHO OF THE DECLARATION IS NOT.
    //
    // Below this used to be, unconditionally, `echo 'command=<the YAML text>'`
    // — a PURE FUNCTION OF THE DECLARATION. It cannot ever differ from the
    // config it was generated from, so hashing it answers "is the config the
    // config". A task could never drift, by construction.
    //
    // Measured on the paiml fleet: 151 task resources, ZERO with
    // `output_artifacts`, so every one of them had this observable — while 186
    // `completion_check`s sat there being called by the apply path and by
    // nothing else. Those resources carry the fleet's network configuration.
    //
    // `check_script` already turns a completion_check into
    // `verdict::single(check, "task=completed", "task=pending")`, which asks
    // the HOST. The generator was never the problem; nothing on the drift path
    // called it. (forjar#279.)
    //
    // Ordering note: `output_artifacts` still wins above, because artifact
    // digests distinguish WHICH output changed, where a completion_check only
    // says completed-or-not.
    if let Some(ref check) = resource.completion_check {
        return verdict::single(check, "task=completed", "task=pending");
    }

    // No artifacts and no completion_check: there is genuinely nothing to
    // observe. Say so rather than echoing the declaration back — a caller can
    // tell "unobservable" from "unchanged", which an echo cannot.
    // `printf '%s\n'`, not `echo`: dash's XSI echo expands the `\n` that
    // `render_command_inline` puts in, so the sentinel's stdout — which drift
    // HASHES — would differ between a bash target and a dash target and
    // manufacture drift from nothing. (#350)
    let command = resource.command.as_deref().unwrap_or("true");
    format!(
        "printf '%s\\n' {}",
        sh_squote(&format!(
            "unobservable:no-completion-check:{}",
            render_command_inline(command)
        ))
    )
}

/// FJ-2704: Generate shell script to scatter local artifacts to remote paths.
///
/// Each scatter entry is a "local:remote" mapping. Returns a script that copies
/// local files to their remote destinations before task execution.
pub fn scatter_script(resource: &Resource) -> Option<String> {
    if resource.scatter.is_empty() {
        return None;
    }
    let mut script = String::from("set -euo pipefail\n# FJ-2704: scatter artifacts\n");
    for mapping in &resource.scatter {
        if let Some((local, remote)) = mapping.split_once(':') {
            let (l, r) = (sh_squote(local), sh_squote(remote));
            script.push_str(&format!("mkdir -p \"$(dirname {r})\"\ncp -r {l} {r}\n"));
        }
    }
    Some(script)
}

/// FJ-2704: Generate shell script to gather remote artifacts to local paths.
///
/// Each gather entry is a "remote:local" mapping. Returns a script that copies
/// remote files to their local destinations after task execution.
pub fn gather_script(resource: &Resource) -> Option<String> {
    if resource.gather.is_empty() {
        return None;
    }
    let mut script = String::from("set -euo pipefail\n# FJ-2704: gather artifacts\n");
    for mapping in &resource.gather {
        if let Some((remote, local)) = mapping.split_once(':') {
            let (r, l) = (sh_squote(remote), sh_squote(local));
            script.push_str(&format!("mkdir -p \"$(dirname {l})\"\ncp -r {r} {l}\n"));
        }
    }
    Some(script)
}
