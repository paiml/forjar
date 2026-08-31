//! Observability registry: every desired-state field must be classified.
//!
//! WHY THIS EXISTS.
//!
//! forjar's failure polarity was backwards: a field nobody observed defaulted to
//! CONVERGED. Chef is the mirror image — an unloaded property yields a permanent
//! diff, so blindness makes a resource noisy rather than falsely green. This
//! registry buys that polarity cheaply: a field that participates in the desired
//! state must be declared either observable (with a value to mutate it to, so
//! the behavioural gate can dirty a baseline) or explicitly unobservable with a
//! reason. There is no third option and no default.
//!
//! WHAT MAKES IT A GATE RATHER THAN A DOCUMENT.
//!
//! The set of desired-state fields is discovered by REFLECTION, not from a
//! hand-written list. `hashed_fields()` serializes a Resource, mutates one field
//! at a time, and reports which mutations change `hash_desired_state`. Adding a
//! field to the hash therefore adds it to that set automatically, and
//! `every_hashed_field_is_classified` fails until someone decides what it is.
//!
//! A hand-written list is precisely what this fleet has been bitten by before —
//! `forjar check` passed for five months behind assertions that enumerated
//! fields by hand and silently omitted the interesting ones.

use crate::core::types::Resource;

/// What forjar can find out about a declared field on a real host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observability {
    /// The host can be asked. `alt` is a DIFFERENT valid value, used by the
    /// behavioural gate to seed a present-but-wrong baseline: apply with `alt`,
    /// then apply the declaration, then assert the host matches the
    /// declaration. That sequence is the one Molecule's
    /// destroy→create→converge default skips, and it is the only state in which
    /// the 2026-08-19 mount bug was visible.
    Observed { alt: &'static str },
    /// There is genuinely nothing to observe. An action has no converged form;
    /// `output_artifacts` is the honest answer for those. A reason is REQUIRED
    /// so this cannot become a shrug.
    Unobservable(&'static str),
    /// Observable in principle, not yet done. Carries a ticket so the backlog is
    /// countable rather than implied.
    Unmigrated(&'static str),
}

/// Classify one desired-state field.
///
/// Deliberately keyed on the FIELD NAME alone rather than (type, field). Twenty
/// resource types by fourteen fields is 280 arms that one operator will not
/// maintain, and the per-type nuance that actually matters lives in the
/// behavioural gate's fixtures, not here. What this table has to guarantee is
/// narrower and sufficient: no hashed field is unclassified.
pub fn classify(field: &str) -> Option<Observability> {
    Some(match field {
        // Read back directly from the host by an existing check or state_query.
        "path" => Observability::Observed {
            alt: "/tmp/forjar-alt-path",
        },
        "content" => Observability::Observed {
            alt: "forjar-alt-content",
        },
        "source" => Observability::Observed {
            alt: "//forjar-alt/share",
        },
        "fs_type" => Observability::Observed { alt: "nfs" },
        "options" => Observability::Observed { alt: "ro" },
        "owner" => Observability::Observed { alt: "nobody" },
        "group" => Observability::Observed { alt: "nogroup" },
        "mode" => Observability::Observed { alt: "0600" },
        "version" => Observability::Observed {
            alt: "0.0.1-forjar-alt",
        },
        "packages" => Observability::Observed {
            alt: "forjar-alt-package",
        },
        "state" => Observability::Observed { alt: "absent" },
        "target" => Observability::Observed {
            alt: "/tmp/forjar-alt-target",
        },

        // `provider` selects HOW to converge, not WHAT state to reach. Two
        // resources differing only in provider describe the same host state by
        // different means, so there is nothing on the host that distinguishes
        // them. It is in the hash because changing it must re-apply.
        "provider" => Observability::Unobservable(
            "selects the mechanism, not the state; the host cannot report which \
             provider installed a thing",
        ),
        // `name` is an identity/label for several types rather than host state.
        "name" => Observability::Unobservable("an identifier, not a property of the host"),

        // ── Phase-2 fields ────────────────────────────────────────────────
        // Reflection found these; `collect_core_fields` is only half the
        // desired-state surface. That gap is itself the argument for discovering
        // the set rather than listing it: a hand-maintained registry would have
        // covered fourteen fields and quietly ignored seventeen more.
        "fstype" => Observability::Observed { alt: "nfs" },
        "shell" => Observability::Observed {
            alt: "/usr/sbin/nologin",
        },
        "home" => Observability::Observed {
            alt: "/tmp/forjar-alt-home",
        },
        "image" => Observability::Observed {
            alt: "forjar/alt-image:0",
        },
        "ports" => Observability::Observed { alt: "9999:9999" },
        "port" => Observability::Observed { alt: "9999" },
        "volumes" => Observability::Observed {
            alt: "/tmp/forjar-alt-vol",
        },
        "environment" => Observability::Observed {
            alt: "FORJAR_ALT=1",
        },
        "restart" => Observability::Observed { alt: "no" },
        "schedule" => Observability::Observed { alt: "0 0 31 2 *" },
        "command" => Observability::Observed {
            alt: "true # forjar-alt",
        },
        "overlay_ip" => Observability::Observed { alt: "10.42.0.254" },
        "overlay_iface" => Observability::Observed { alt: "forjar-alt0" },
        "protocol" => Observability::Observed { alt: "udp" },
        "from" => Observability::Observed {
            alt: "10.42.0.0/24",
        },
        // #390: a task's assertion. The host can be asked directly — it's the
        // one field whose "observation" IS running it (task.rs's check_script
        // wraps it in the verdict harness apply already runs). `alt` is a
        // check that can never pass, so the behavioural gate can seed a
        // present-but-violated baseline distinct from the declared one.
        "completion_check" => Observability::Observed {
            alt: "false # forjar-alt-completion-check",
        },

        // `action` and `restart_on` describe WHEN/HOW to act, not a state the
        // host holds. A host cannot report which trigger caused a restart.
        "action" => Observability::Unobservable(
            "names an operation to perform, not a state the host can report",
        ),
        "restart_on" => Observability::Unobservable(
            "a trigger condition, not observable state; the host cannot say what \
             would have restarted it",
        ),

        _ => return None,
    })
}

/// Every field whose mutation changes `hash_desired_state`, found by reflection.
///
/// Serializes a baseline Resource, and for each JSON field substitutes a marker
/// value and re-hashes. A field whose mutation moves the hash is, by
/// definition, part of the desired state and must be classified.
///
/// This is the forcing function. It cannot go stale the way a hand-written list
/// does, because it asks the hasher rather than a human.
pub fn hashed_fields() -> Vec<String> {
    use crate::core::planner::hashing::hash_desired_state;

    let base = Resource::default();
    let base_hash = hash_desired_state(&base);
    let Ok(serde_json::Value::Object(map)) = serde_json::to_value(&base) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for key in map.keys() {
        let mut probe = map.clone();
        // A string is accepted by Option<String>, Vec<String> (as one element)
        // and most scalar fields; anything that fails to deserialize simply does
        // not get probed, which is safe — it cannot silently claim "unhashed".
        probe.insert(key.clone(), serde_json::json!("forjar-probe-sentinel"));
        let as_list = {
            let mut p = map.clone();
            p.insert(key.clone(), serde_json::json!(["forjar-probe-sentinel"]));
            p
        };
        for candidate in [probe, as_list] {
            if let Ok(r) = serde_json::from_value::<Resource>(serde_json::Value::Object(candidate))
            {
                if hash_desired_state(&r) != base_hash {
                    out.push(key.clone());
                    break;
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests;
