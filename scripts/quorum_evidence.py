#!/usr/bin/env python3
"""Quorum evidence checks -- Refs #390.

WHY THIS EXISTS. The owner's rule is that a quorum's intermediate results AND its
conclusion must be attached to the PR as evidence and metadata. Audited on the
gate's own first PR, that was not happening: the receipt carried the 17 REFUTED
claim texts and the 43 CONFIRMED ones existed only as the integer 43, while the
lane summaries, judge scores and independent review lived in agent-local state
(~36 MB) and /tmp (~9.6 MB) -- both gone on reboot. A verdict whose evidence
evaporates is an assertion.

WHAT IT PROVES, AND WHAT IT DOES NOT. It proves that scrubbed, untruncated,
diff-cited, unrecycled prose is COMMITTED, matches the tallies, and cites lines
that resolve at the commit the quorum actually read. It does NOT prove a quorum
happened -- nothing here observes seven blind lanes or three judges. On-target
fabrication was priced during design, not waved away: a ~40-line template loop
still passes. This raises the floor from "type an integer" to "write reviewable
prose anchored to the real diff"; it is not a proof of process.

OFFLINE BY CONSTRUCTION. git object DB + re + hashlib. A pre-push hook must never
need the network.
"""
import hashlib
import json
import re
import subprocess
import sys

FILE_MAX, TOTAL_MAX, ITEM_MIN, ANCHOR_MIN = 128 * 1024, 512 * 1024, 180, 0.33
# Round numbers a writer picks as a budget. A body landing EXACTLY on one is a
# truncation tell, not a coincidence -- 18 of 43 quotes in the first digest were
# exactly 600 bytes.
TRUNC_BUDGETS = (200, 300, 400, 500, 600, 800, 1000, 1200, 1400, 1500, 2000, 4000)
ROLES = {"claims", "lanes", "judges", "agy", "pmat", "proposals", "crux"}

SEC_RE = re.compile(r"(?m)^#{2,4}\s+(CONFIRMED|REFUTED)\b")
ITEM_RE = re.compile(r"(?m)^(?=\d+\.\s+\[)")
STOP_RE = re.compile(r"(?m)^(?:\d+\.\s+\[|#)")
SUB_RE = re.compile(r"(?m)^\s*-\s+(evidence|corrected):\s*(.*)$")
# A RELEASE COMMIT MUST BE ABLE TO CITE ITSELF. This matched Rust sources only,
# so the three files a release edits -- Cargo.toml, Cargo.lock, CHANGELOG.md --
# were not citations at all, and v1.25.2's receipt anchored 0 of 19 claims.
# v1.25.0 (0b4f2e3e) and v1.25.1 (813159f2) were pushed `waived` for the same
# reason: the gate was refusing a SHAPE rather than bad evidence, and a gate that
# cannot be passed honestly is what teaches a repo to reach for the waiver.
# The root files are matched AT THE ROOT ONLY -- the lookbehind refuses
# `crates/x/Cargo.toml:3`, which would resolve to a different file than it names.
CIT_RE = re.compile(
    r"\b((?:src|tests|scripts|benches)/[A-Za-z0-9_./-]+\.rs"
    r"|(?<![\w./-])(?:Cargo\.toml|Cargo\.lock|CHANGELOG\.md|README\.md)):(\d+)\b"
)

# SHAPE-BASED, never environment-derived. A scanner keyed on "whoever is running
# it" passes clean on every other machine. These sixteen are a DENYLIST and
# cannot prove absence -- a novel credential shape or a customer name in prose
# walks straight through. Named as a limit rather than sold as a guarantee.
LEAKS = [
    ("user_at_ip", re.compile(r"\b[A-Za-z0-9._-]+@(?:\d{1,3}\.){3}\d{1,3}\b")),
    ("sshd_user", re.compile(r"sshd:\s*(?!<)[A-Za-z0-9._-]+@")),
    ("ssh_controlpath", re.compile(r"ControlPath=(?!<)\S+")),
    ("home_path", re.compile(r"/(?:home|Users)/(?!<)[A-Za-z0-9._-]+")),
    ("mac_scratch", re.compile(r"/var/folders/[A-Za-z0-9+_/-]{6,}")),
    ("claude_scratch", re.compile(r"/tmp/claude-\d+")),
    ("session_uuid", re.compile(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b")),
    ("aws_access_key", re.compile(r"AKIA[0-9A-Z]{16}")),
    ("github_token", re.compile(r"gh[pso]_[A-Za-z0-9_]{36,}")),
    ("private_key", re.compile(r"-----BEGIN (?:RSA|EC|DSA|OPENSSH) PRIVATE KEY-----")),
    ("stripe_key", re.compile(r"[sr]k_(?:live|test)_[A-Za-z0-9]{20,}")),
    ("age_secret", re.compile(r"AGE-SECRET-KEY-1[A-Z0-9]{58}")),
    ("jwt", re.compile(r"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}")),
    ("db_url_creds", re.compile(r"(?i)(?:mysql|postgres|postgresql|mongodb)://[^\s:/]+:[^\s@]+@")),
    ("slack_webhook", re.compile(r"https://hooks\.slack\.com/services/\S+")),
]


def die(msg):
    print(f"\u2717 QUORUM GATE: {msg}", file=sys.stderr)
    sys.exit(1)


def git(*args, binary=False):
    p = subprocess.run(["git", *args], capture_output=True)
    return p.returncode, (p.stdout if binary else p.stdout.decode("utf-8", "replace"))


def blob_at(rev, path):
    rc, out = git("cat-file", "blob", f"{rev}:{path}", binary=True)
    return out if rc == 0 else None


def sections(text):
    heads = list(SEC_RE.finditer(text))
    for i, h in enumerate(heads):
        end = heads[i + 1].start() if i + 1 < len(heads) else len(text)
        yield h.group(1), text[h.end():end]


def items(body):
    out = []
    for part in ITEM_RE.split(body):
        if not re.match(r"\d+\.\s+\[", part):
            continue
        # TIGHT bound. Without it the last item of a section absorbs everything
        # after it and passes every length floor for free.
        m = STOP_RE.search(part, 1)
        out.append(part[:m.start()] if m else part)
    return out


def truncated(body):
    n = len(body)
    if n in TRUNC_BUDGETS:
        return f"length is exactly {n} -- a writer budget, not a sentence"
    if body.count("`") % 2 == 1:
        return "ends inside an unclosed backtick span"
    # A THIRD RULE WAS TRIED AND DROPPED: `[A-Za-z0-9_/]$` -> "ends mid-token".
    # Measured against the real #390 corpus it fired on COMPLETE prose -- a 639 B
    # body, well under its 1337 B budget, genuinely ending `bash-exit=127`. Plenty
    # of honest sentences end in a number or an identifier. The two rules above are
    # high precision and caught every actual defect in that corpus (18 bodies at
    # exactly 600 B, 11 unclosed backticks); the third added false refusals and no
    # detection. A gate that cries wolf on good evidence trains people to bypass it.
    return None


def check_manifest(ev, head, blobs):
    listed, roles, total = set(), set(), 0
    for e in ev["files"]:
        path, rs = e.get("path", ""), e.get("roles", [])
        if not path.startswith(".quorum/evidence/"):
            die(f"evidence path '{path}' is outside .quorum/evidence/")
        if path in listed:
            die(f"evidence path '{path}' is listed twice")
        if not isinstance(rs, list) or not rs or set(rs) - ROLES:
            die(f"evidence '{path}' roles {rs!r}; must be a non-empty subset of {sorted(ROLES)}")
        listed.add(path)
        roles.update(rs)
        raw = blob_at(head, path)
        if raw is None:
            die(f"evidence '{path}' is NOT COMMITTED at HEAD.\n"
                f"  The gate reads the tree; CI reviews what was PUSHED.\n"
                f"  Commit it:  git add {path} && git commit")
        bid = git("rev-parse", f"{head}:{path}")[1].strip()
        sha = hashlib.sha256(raw).hexdigest()
        for field, got, want in (("blob", e.get("blob"), bid),
                                 ("sha256", e.get("sha256"), sha),
                                 ("bytes", e.get("bytes"), len(raw))):
            if got != want:
                die(f"evidence '{path}': receipt {field}={str(got)[:16]}, committed {str(want)[:16]}")
        if len(raw) > FILE_MAX:
            die(f"evidence '{path}' is {len(raw)}B over the {FILE_MAX}B ceiling. Split by role;\n"
                "  the raw journal belongs in an expiring artifact, not in git.")
        blobs[path] = (bid, raw)
        total += len(raw)
    if total != ev.get("total_bytes"):
        die(f"evidence.total_bytes={ev.get('total_bytes')}, committed blobs sum to {total}")
    if total > TOTAL_MAX:
        die(f"evidence totals {total}B over the {TOTAL_MAX}B ceiling")
    for need in ("claims", "lanes", "judges", "agy"):
        if need not in roles:
            die(f"no evidence file carries role '{need}'. The rule names lane summaries,\n"
                "  judge scores and the independent review as evidence -- not only claims.")
    return listed


def check_provenance(listed, blobs, touched):
    """Anchored to things the pusher did not author: the diff, and origin/main."""
    if not (listed & touched):
        die("no evidence file is touched by this branch. Every listed file already\n"
            "  existed unchanged -- which is exactly what a recycled receipt looks like.")
    rc, out = git("ls-tree", "-r", "origin/main", "--", ".quorum/evidence")
    prior = {ln.split()[2] for ln in out.splitlines() if len(ln.split()) > 2}
    for path, (bid, _) in blobs.items():
        if bid in prior:
            die(f"evidence '{path}' is byte-identical to a blob already on origin/main.\n"
                "  Copying a merged PR's evidence forward is the cheapest forgery there is.")


def check_redaction(blobs):
    for path, (_, raw) in blobs.items():
        text = raw.decode("utf-8", "replace")
        for name, rx in LEAKS:
            m = rx.search(text)
            if m:
                line = text[:m.start()].count("\n") + 1
                die(f"'{path}':{line} leaks {name}: {m.group(0)[:48]!r}\n"
                    "  Scrub it (<user>, <host>, <SCRATCH>, <REPO>) and re-commit.\n"
                    "  THIS REPO IS PUBLIC -- this exact class already leaked once.")


def check_claims(blobs, dp, want_conf, want_ref, base, head, touched):
    text = blobs[dp][1].decode("utf-8", "replace")
    counts, seen, adjudicated = {"CONFIRMED": 0, "REFUTED": 0}, set(), []
    for kind, body in sections(text):
        for it in items(body):
            counts[kind] += 1
            adjudicated.append(it)
            head_line = re.sub(r"\s+", " ", it.split("\n")[0]).strip().lower()
            if head_line in seen:
                die(f"'{dp}': duplicate claim {head_line[:60]!r} -- "
                    "N copies of one sentence is a tally with extra steps")
            seen.add(head_line)
            if len(it) < ITEM_MIN:
                die(f"'{dp}': a {kind} claim is {len(it)}B, floor {ITEM_MIN}B")
            subs = SUB_RE.findall(it)
            if not subs:
                die(f"'{dp}': a {kind} claim carries no '- evidence:'/'- corrected:' subline")
            for _, sb in subs:
                why = truncated(sb.rstrip())
                if why:
                    die(f"'{dp}': a {kind} subline is TRUNCATED -- {why}.\n"
                        f"     ...{sb.rstrip()[-60:]!r}\n"
                        "  Fix the EMITTER: a severed citation is unreviewable and no\n"
                        "  other tier survives to complete it.")
    # SYMMETRY. The gate used to demand prose for what the panel KILLED and accept
    # a bare integer for what it BLESSED -- strict about the claims nobody
    # fabricates to look good, lax about the ones that ship into the changelog.
    for kind, want, label in (("CONFIRMED", want_conf, "claims_confirmed"),
                              ("REFUTED", want_ref, "claims_refuted")):
        if counts[kind] != want:
            die(f"{label}={want} but the digest carries {counts[kind]} {kind} claims.\n"
                "  A tally that disagrees with its own prose is the black box this gate\n"
                "  already rejects for the refuted side.")
    check_anchors(adjudicated, dp, base, head, touched)


def anchors_at_base(src, dp, p, n, touched):
    """Whether a citation that RESOLVES AT THE MERGE BASE anchors its claim.

    This is the strong form. The cited text sits in a tree the pusher did not
    author, so a line number cannot be invented to fit a claim. An out-of-range
    citation is refused BY NAME rather than quietly dropped: a citation the gate
    cannot check is not a citation the gate should count.
    """
    if int(n) > src.count(b"\n") + 1:
        die(f"'{dp}': cites '{p}:{n}' but that file has {src.count(chr(10).encode())+1} lines at base")
    return p in touched


def anchors_as_added(head, dp, p, n, touched):
    """Whether a citation into a file THIS BRANCH ADDS anchors its claim.

    The guarantee is weaker than `anchors_at_base`'s and worth naming rather
    than glossing. It cannot be "resolves against a tree the pusher did not
    author" -- the pusher wrote every line of it. What remains mechanically
    checkable is that the cited line EXISTS in the commit being pushed: the line
    is in the diff by construction, so the citation names real, reviewable text
    and a fabricated line number is refused BY NAME, the same treatment the base
    rule has always given an out-of-range citation.

    Refusing the whole shape instead was measured: it left a release commit --
    Cargo.toml, Cargo.lock, CHANGELOG.md and one new test -- with nothing it
    could cite. A path that is neither at base nor in the diff is neither this
    branch's work nor the tree it started from, and anchors nothing.
    """
    if p not in touched:
        return False
    added = blob_at(head, p)
    if added is None:
        return False
    if int(n) > added.count(b"\n") + 1:
        die(f"'{dp}': cites '{p}:{n}' but that file, which this branch "
            f"ADDS, has {added.count(chr(10).encode())+1} lines at HEAD")
    return True


def check_anchors(adjudicated, dp, base, head, touched):
    """A citation must resolve IN THE TREE and name a file this diff touches.

    This is the check with teeth, because it is anchored to the merge-base tree --
    something the pusher did not author. It is the free-rider defence the gate
    already applies to falsification.test_file, extended from the test to the
    reasoning. File-level, not hunk-level: hunk-level was measured at 25% on real
    data and would have failed honest work.

    Two resolutions anchor and they carry different guarantees: `anchors_at_base`
    is the strong one, `anchors_as_added` the weaker one a release commit needs
    in order to be able to cite itself. Both are consulted for EVERY citation,
    never short-circuited, so an out-of-range line number is still refused once a
    claim is already anchored by an earlier citation.
    """
    anchored = 0
    for it in adjudicated:
        hit = False
        for p, n in CIT_RE.findall(it):
            src = blob_at(base, p)
            if src is None:
                hit = anchors_as_added(head, dp, p, n, touched) or hit
            else:
                hit = anchors_at_base(src, dp, p, n, touched) or hit
        anchored += 1 if hit else 0
    if adjudicated:
        rate = anchored / len(adjudicated)
        if rate < ANCHOR_MIN:
            die(f"only {anchored}/{len(adjudicated)} ({rate:.0%}) adjudicated claims cite a\n"
                f"  file:line inside this branch's own diff; floor is {ANCHOR_MIN:.0%}.\n"
                "  Prose about code the branch never touched is not evidence FOR this change.")


def main():
    receipt, want_conf, want_ref, touched_raw, base, head = sys.argv[1:7]
    want_conf, want_ref = int(want_conf), int(want_ref)
    touched = set(filter(None, touched_raw.splitlines()))
    try:
        r = json.load(open(receipt))
    except Exception as e:
        die(f"{receipt} is not valid JSON: {e}")

    # ONLY base_commit is recorded. `head_commit` was in the design and is
    # SELF-REFERENTIAL: committing the receipt changes HEAD, which invalidates the
    # head_commit the receipt just recorded, forever. Blob ids are safe in the
    # manifest because git blobs are content-addressed and do not move when a new
    # commit is made -- HEAD is not. Nothing is lost: `diff_sha256` already binds
    # the content of HEAD, which is what head_commit was reaching for.
    got = r.get("base_commit")
    if not got:
        die(f"receipt is missing 'base_commit'. Without it every file:line citation\n"
            f"  rots the moment this branch's own fix moves a line. Set it to {base}.")
    if got != base:
        die(f"receipt base_commit={got[:12]}... but the merge-base is {base[:12]}...")

    ev = r.get("evidence")
    if not isinstance(ev, dict) or not ev.get("files"):
        die("receipt has no evidence.files[]. A verdict with no attached reasoning is\n"
            "  the bare integer the owner's rule rejects.")

    blobs = {}
    listed = check_manifest(ev, head, blobs)
    check_provenance(listed, blobs, touched)
    check_redaction(blobs)
    dp = ev.get("claims_digest")
    if dp not in blobs:
        die("evidence.claims_digest must name one of evidence.files[]")
    check_claims(blobs, dp, want_conf, want_ref, base, head, touched)
    print(f"  evidence: {len(blobs)} files, {ev['total_bytes']}B, "
          f"{want_conf} confirmed + {want_ref} refuted, redaction clean")


if __name__ == "__main__":
    main()
