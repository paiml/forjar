//! Script generators for `ec2_spot_seat`. Every value reaching a command line has
//! passed `spec_of` and is single-quoted anyway; the image id, tar key and tar
//! sha256 are read from the pin file at generation time and nowhere else.

use super::{parse_pin, spec_of, Phase, Pin, SpotSeat};
use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::verdict;

pub(super) fn reject(msg: &str) -> String {
    format!("echo {} >&2; exit 1", sh_squote(&format!("REFUSED: {msg}")))
}

/// Shell function `say`: one line to stderr and to the box's append-only log.
pub(super) const SAY: &str = "LOG=\"${FORJAR_SPOT_LOG:-/var/log/forjar-spot-seat.log}\"\n\
say() { printf '%s %s\\n' \"$(date -u +%FT%TZ)\" \"$*\" | tee -a \"$LOG\" >&2; }\n";

pub(super) fn pin_of(s: &SpotSeat) -> Result<Pin, String> {
    let text = std::fs::read_to_string(&s.pin_file)
        .map_err(|e| format!("pin file {} unreadable: {e}", s.pin_file))?;
    parse_pin(&text, &s.pin_id_key, &s.pin_s3_key_key, &s.pin_sha256_key)
}

fn image_present(p: &Pin) -> String {
    format!("docker image inspect {} >/dev/null 2>&1", sh_squote(&p.id))
}

/// On-box: fetch, sha256-check, load and inspect the pinned image.
fn image_load(s: &SpotSeat, p: &Pin) -> Result<String, String> {
    let bucket = s
        .image_bucket
        .as_deref()
        .ok_or("image-load needs `spot_image_bucket`")?;
    let src = sh_squote(&format!("s3://{bucket}/{}", p.s3_key));
    Ok(format!(
        "set -u\n{SAY}IMG={img}\nWORK=\"${{FORJAR_SPOT_WORK:-/work}}\"; TAR=\"$WORK/seat-image.tar.zst\"\n\
         if docker image inspect \"$IMG\" >/dev/null 2>&1; then say \"image-present:$IMG\"; exit 0; fi\n\
         mkdir -p \"$WORK\"\n\
         aws s3 cp {src} \"$TAR\" --region {region} >/dev/null || {{ say 'image-fetch-failed:'{src}; exit 1; }}\n\
         printf '%s  %s\\n' {sha} \"$TAR\" | sha256sum -c >/dev/null 2>&1 || {{ say 'image-sha256-mismatch:'{key}; rm -f \"${{TAR:?}}\"; exit 1; }}\n\
         zstd -dc \"$TAR\" | docker load >\"$WORK/load.log\" 2>&1 || {{ say \"image-load-failed: $(tail -n 3 \"$WORK/load.log\" | tr '\\n' ' ')\"; exit 1; }}\n\
         rm -f \"${{TAR:?}}\"\n\
         docker image inspect \"$IMG\" >/dev/null 2>&1 || {{ say \"image-missing:$IMG have: $(docker image ls -a --no-trunc -q | tr '\\n' ' ')\"; exit 1; }}\n\
         say \"image-loaded:$IMG\"\n",
        img = sh_squote(&p.id),
        sha = sh_squote(&p.sha256),
        key = sh_squote(&p.s3_key),
        region = sh_squote(&s.region),
    ))
}

pub(super) fn jit_dir(s: &SpotSeat) -> String {
    s.jit_source
        .strip_prefix("dir:")
        .map_or_else(|| "/run/forjar/spot-jit".to_string(), str::to_string)
}

/// On-box: one container per JIT config, one log per seat, non-zero when a seat
/// never reaches `running`.
fn seat_start(s: &SpotSeat, p: &Pin) -> String {
    format!(
        "set -u\n{SAY}IMG={img}\nOWNER={owner}\nN={n}\nJITDIR={jit}\n\
         LOGDIR=\"${{FORJAR_SPOT_SEATLOGS:-/var/log/forjar-seats}}\"; WAIT=\"${{FORJAR_SPOT_WAIT:-120}}\"; POLL=\"${{FORJAR_SPOT_POLL:-2}}\"\n\
         mkdir -p \"$LOGDIR\"\n\
         docker image inspect \"$IMG\" >/dev/null 2>&1 || {{ say \"seat-image-missing:$IMG\"; exit 1; }}\n\
         rc=0; i=1\n\
         while [ \"$i\" -le \"$N\" ]; do\n  \
           name=\"$(printf 'seat-%s-%02d' \"$OWNER\" \"$i\")\"; log=\"$LOGDIR/$name.log\"; jit=\"$JITDIR/$name.jit\"\n  \
           if [ \"$(docker inspect -f '{{{{.State.Running}}}}' \"$name\" 2>/dev/null)\" = true ]; then say \"seat-running:$name\"; i=$((i+1)); continue; fi\n  \
           [ -s \"$jit\" ] || {{ say \"seat-jit-missing:$name\"; echo \"seat-jit-missing:$jit\" >\"$log\"; rc=1; i=$((i+1)); continue; }}\n  \
           docker rm -f \"$name\" >/dev/null 2>&1\n  \
           docker run -d --name \"$name\" --label \"forjar.seat=$name\" --pids-limit 16384 --security-opt=no-new-privileges \\\n    \
             --entrypoint=env --user=1001:1001 -eHOME=/home/runner --tmpfs=/tmp:exec,mode=1777 -v \"$jit:/run/seat.jit:ro\" \"$IMG\" \\\n    \
             sh -c 'ACTIONS_RUNNER_INPUT_JITCONFIG=$(cat /run/seat.jit); export ACTIONS_RUNNER_INPUT_JITCONFIG; exec /home/runner/run.sh' >\"$log\" 2>&1 || {{ say \"seat-docker-run-failed:$name: $(tail -n 3 \"$log\" | tr '\\n' ' ')\"; rc=1; }}\n  \
           i=$((i+1))\n\
         done\n\
         i=1\n\
         while [ \"$i\" -le \"$N\" ]; do\n  \
           name=\"$(printf 'seat-%s-%02d' \"$OWNER\" \"$i\")\"; log=\"$LOGDIR/$name.log\"; t=0\n  \
           until [ \"$(docker inspect -f '{{{{.State.Running}}}}' \"$name\" 2>/dev/null)\" = true ]; do\n    \
             if [ \"$t\" -ge \"$WAIT\" ]; then docker logs --tail 40 \"$name\" >>\"$log\" 2>&1; say \"seat-never-running:$name log=$log tail: $(tail -n 3 \"$log\" | tr '\\n' ' ')\"; rc=1; break; fi\n    \
             sleep \"$POLL\"; t=$((t+POLL))\n  \
           done\n  \
           [ \"$t\" -lt \"$WAIT\" ] && say \"seat-running:$name\"\n  \
           i=$((i+1))\n\
         done\n\
         exit \"$rc\"\n",
        img = sh_squote(&p.id),
        owner = sh_squote(&s.owner),
        n = s.seats,
        jit = sh_squote(&jit_dir(s)),
    )
}

fn running_count() -> &'static str {
    "docker ps -q --filter 'label=forjar.seat' --filter status=running 2>/dev/null | wc -l | tr -d ' '"
}

fn parts(resource: &Resource) -> Result<(SpotSeat, Option<Pin>), String> {
    let s = spec_of(resource)?;
    let pin = if matches!(s.phase, Phase::ImageLoad | Phase::SeatStart) {
        Some(pin_of(&s)?)
    } else {
        None
    };
    Ok((s, pin))
}

/// Read-only: does the box match the declared phase?
pub fn check_script(resource: &Resource) -> String {
    let (s, pin) = match parts(resource) {
        Ok(v) => v,
        Err(e) => return reject(&e),
    };
    let a = match (s.phase, &pin) {
        (Phase::ImageLoad, Some(p)) => verdict::assert_that(
            &image_present(p),
            &format!("spot-image-present:{}", p.id),
            &format!("spot-image-missing:{}", p.id),
        ),
        (Phase::SeatStart, _) => verdict::assert_that(
            &format!("[ \"$({})\" -ge {} ]", running_count(), s.seats),
            &format!("spot-seats-running:{}", s.seats),
            "spot-seats-short",
        ),
        _ => return super::script_ctl::check(&s),
    };
    verdict::check_script_from(&[a])
}

/// Read-only: one line of observed state.
pub fn state_query_script(resource: &Resource) -> String {
    let (s, pin) = match parts(resource) {
        Ok(v) => v,
        Err(e) => return reject(&e),
    };
    match (s.phase, &pin) {
        (Phase::ImageLoad, Some(p)) => format!(
            "if {}; then echo 'image=present'; else echo 'image=absent'; fi\n",
            image_present(p)
        ),
        (Phase::SeatStart, _) => format!("echo \"seats_running=$({})\"\n", running_count()),
        _ => super::script_ctl::state_query(&s),
    }
}

/// Converges the declared phase; every failure is a non-zero exit plus a log line.
pub fn apply_script(resource: &Resource) -> String {
    let (s, pin) = match parts(resource) {
        Ok(v) => v,
        Err(e) => return reject(&e),
    };
    let out = match (s.phase, &pin) {
        (Phase::ImageLoad, Some(p)) => image_load(&s, p),
        (Phase::SeatStart, Some(p)) => Ok(seat_start(&s, p)),
        _ => super::script_ctl::apply(&s),
    };
    out.unwrap_or_else(|e| reject(&e))
}
