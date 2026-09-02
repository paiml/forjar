//! #403 (audit E01): the fields the desired-state hash gained when it stopped
//! being an allowlist.
//!
//! `hash_desired_state` used to cover 35 of `Resource`'s 122 serialised fields.
//! Folding in the rest is what makes a changed `uid`, `tag`, `checksum` or
//! `driver_version` re-converge instead of being reported `unchanged` — and it
//! immediately made `observe::tests::every_hashed_field_is_classified` fail
//! with 45 newly-hashed fields, exactly as that gate is designed to.
//!
//! Every arm below is a DECISION about one field, not a shrug:
//!
//!   * `Observed` — a `state_query_script` in `src/resources/` genuinely reads
//!     this back. Each one here was checked against the generator, not guessed.
//!   * `Unobservable` — the field names an action, a route or a selector. There
//!     is no fact on the host that could disagree with it.
//!   * `Unmigrated` — observable in principle, and forjar does not ask. These
//!     are the countable backlog; the reason says what query would close it.
//!
//! Split out of `mod.rs` so that file stays inside the 500-line health limit
//! and so each group below stays a small function rather than one 45-arm match.

use super::Observability;

/// Chain the per-family tables. Order is irrelevant — the field sets are
/// disjoint — so this stays a flat fold rather than a precedence rule.
pub(super) fn classify(field: &str) -> Option<Observability> {
    gpu_and_model(field)
        .or_else(|| release_and_namespace(field))
        .or_else(|| task_and_hooks(field))
        .or_else(|| storage(field))
}

/// `type: gpu` and `type: model`.
fn gpu_and_model(field: &str) -> Option<Observability> {
    Some(match field {
        // `gpu`'s state query BRANCHES on the backend and reports which stack
        // answered (`cpu-only`, rocminfo, nvidia-smi), so the host does say
        // which one is installed.
        "gpu_backend" => Observability::Observed { alt: "rocm" },
        // nvidia-smi --query-gpu=driver_version is literally in the query.
        "driver_version" => Observability::Observed { alt: "525" },
        // ...as is compute_mode, in the same csv row.
        "compute_mode" => Observability::Observed { alt: "prohibited" },
        // The rocm branch reads /sys/module/amdgpu/version — the KERNEL MODULE
        // version, which is a different fact from the ROCm stack version this
        // field declares. Reporting a near-miss as "observed" is how a check
        // passes while comparing the wrong thing.
        "rocm_version" => Observability::Unmigrated(
            "#403 — the rocm query reports the amdgpu kernel module version, not \
             the ROCm stack version; `rocminfo`/`hipconfig --version` would",
        ),
        // nvidia-smi reports the DRIVER. The CUDA toolkit version needs
        // `nvcc --version`, which nothing runs.
        "cuda_version" => Observability::Unmigrated(
            "#403 — the nvidia query reports the driver version; the CUDA toolkit \
             version needs `nvcc --version`, which no state query runs",
        ),
        // `model`'s state query b3sums the file at `path`.
        "checksum" => Observability::Observed {
            alt: "blake3:0000000000000000000000000000000000000000000000000000000000000000",
        },
        // The bytes change with the quantization, and the query hashes the
        // bytes — but nothing NAMES the quantization, so a wrong declaration
        // over a right file is indistinguishable from the reverse.
        "quantization" => Observability::Unmigrated(
            "#403 — a GGUF file does not report the quantization it was built \
             with; the content hash moves but nothing names it",
        ),
        "format" => Observability::Unmigrated(
            "#403 — the model query hashes the file and never names its format; \
             reading the container magic would",
        ),
        // The query works off `path`, not `cache_dir`.
        "cache_dir" => Observability::Unmigrated(
            "#403 — the model state query stats `path`; nothing asks where the \
             cache root is, so a moved cache is invisible",
        ),
        _ => return None,
    })
}

/// `type: github_release`, `type: user`, `type: pepita`, `type: recipe`.
fn release_and_namespace(field: &str) -> Option<Observability> {
    Some(match field {
        // github_release's query echoes the repo and stats
        // `{install_dir}/{binary}`, so all three move its output.
        "repo" => Observability::Observed {
            alt: "paiml/forjar-alt",
        },
        "binary" => Observability::Observed {
            alt: "forjar-alt-binary",
        },
        "install_dir" => Observability::Observed {
            alt: "/tmp/forjar-alt-install-dir",
        },
        // An installed binary carries no record of the release it came from.
        // `--version` is a different fact: a rebuilt `nightly` prints the same
        // string it printed yesterday.
        "tag" => Observability::Unobservable(
            "an installed binary carries no record of which release tag produced \
             it; the version it prints is a different fact",
        ),
        "asset_pattern" => Observability::Unobservable(
            "selects WHICH release asset to download; once unpacked the host \
             holds a binary, not the glob that chose it",
        ),
        // `id -Gn` is in user's state query.
        "groups" => Observability::Observed {
            alt: "forjar-alt-group",
        },
        // authorized_keys is a plain readable file — this is a missing query,
        // not an impossible one, and it is the highest-consequence gap here.
        "ssh_authorized_keys" => Observability::Unmigrated(
            "#403 — user's state query reports uid/gid/groups/shell/home and not \
             the key set; ~/.ssh/authorized_keys is readable and should be",
        ),
        // pepita's isolation parameters. /proc/<pid>/root, the mount table and
        // cgroupfs expose every one of them; the query reports only liveness.
        "chroot_dir" | "cpuset" | "overlay_lower" | "overlay_upper" | "overlay_work"
        | "overlay_merged" => Observability::Unmigrated(
            "#403 — a live namespace exposes these through /proc, the mount table \
             and cgroupfs; pepita's state query reports only whether it runs",
        ),
        // A recipe EXPANDS into real resources before anything is applied.
        // Nothing is ever applied under this name.
        "recipe" => Observability::Unobservable(
            "names the recipe that expanded into the real resources; nothing is \
             applied under this name, so no host can be asked about it",
        ),
        _ => return None,
    })
}

/// `type: task`, plus the lifecycle hooks and build routing on every type.
fn task_and_hooks(field: &str) -> Option<Observability> {
    Some(match field {
        // task's state query b3sums each declared artifact when there are any.
        "output_artifacts" => Observability::Observed {
            alt: "/tmp/forjar-alt-artifact",
        },
        "task_inputs" => Observability::Unobservable(
            "declares which files KEY the content-addressed cache; the host holds \
             the files, never the fact that they were the inputs",
        ),
        "ambient_inputs" => Observability::Unobservable(
            "fingerprint commands whose stdout keys the cache — a probe forjar \
             runs, not a state the host retains",
        ),
        "working_dir" => Observability::Unobservable(
            "names the directory a command ran in; nothing on the host records \
             the cwd of a process that has already exited",
        ),
        "quality_gate" => Observability::Unobservable(
            "a pass/fail predicate evaluated during apply; the host keeps no \
             record of which gate was applied to it",
        ),
        "health_check" => Observability::Unobservable(
            "a liveness probe run on a schedule; the host reports whether the \
             service is up, never which probe decided that",
        ),
        "gather" | "scatter" => Observability::Unobservable(
            "controller-side file movement between machines; neither end records \
             that the copy was declared rather than done by hand",
        ),
        "pre_apply" | "post_apply" => Observability::Unobservable(
            "hooks that run around apply; a converged host has no record that a \
             hook ran, only of what it did",
        ),
        "script" => Observability::Unobservable(
            "names the build to run; the host holds the OUTPUT, and a built \
             artifact carries no record of the recipe that produced it",
        ),
        "build_machine" => Observability::Unobservable(
            "names WHICH machine performs the build — controller-side routing \
             that the deploy target cannot report",
        ),
        _ => return None,
    })
}

/// `type: disk_budget`, `type: backup_sync`, `type: nas_archive`.
fn storage(field: &str) -> Option<Observability> {
    Some(match field {
        // disk_budget's query sha256s the deployed .timer unit, whose
        // OnCalendar line IS this value.
        "budget_schedule" => Observability::Observed { alt: "weekly" },
        // backup_sync derives its unit name, script path and heartbeat
        // staleness window from the remote and the schedule, and reports the
        // timer state plus the rclone.conf digest.
        "backup_remote" => Observability::Observed {
            alt: "altdrive:forjar-alt",
        },
        "backup_remote_type" => Observability::Observed { alt: "s3" },
        "backup_schedule" => Observability::Observed { alt: "weekly" },
        // The token is written into rclone.conf, whose sha256 the query
        // reports. forjar compares the DIGEST, never the credential.
        "backup_token" => Observability::Observed {
            alt: "forjar-alt-token",
        },
        // The query proves the sync is installed, timed and healthy — it never
        // asks which roots were declared, so a source dropped from the list
        // looks exactly like a healthy backup.
        "backup_source" => Observability::Unmigrated(
            "#403 — backup_sync's query reports installed/timer/heartbeat but \
             never the declared source roots; the status JSON already carries \
             coverage and could carry the root set",
        ),
        "backup_bandwidth_limit" => Observability::Unmigrated(
            "#403 — the limit is baked into the sync script, and the query \
             digests rclone.conf rather than that script",
        ),
        // nas_archive's query walks the declared dirs and reports
        // archived/pending/absent for each.
        "archive_dirs" => Observability::Observed {
            alt: "forjar-alt-archive-dir",
        },
        "archive_destination" => Observability::Unmigrated(
            "#403 — the archive query reports the SOURCE side only; nothing on \
             the NAS is stat'd, so a wrong destination root is unverified",
        ),
        "archive_schedule" => Observability::Unmigrated(
            "#403 — nas_archive's query has no timer check at all, unlike \
             disk_budget's, so its cadence is unobserved",
        ),
        _ => return None,
    })
}
