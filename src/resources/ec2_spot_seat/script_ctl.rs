//! Controller-side phases of `ec2_spot_seat`: launch, register, teardown. These
//! mirror [`lifecycle`](super::lifecycle) as shell over the `aws` and `gh` CLIs.

use super::script::jit_dir;
use super::{Phase, SpotSeat};
use crate::core::error::ForjarError;
use crate::core::shell_escape::sh_squote;
use crate::resources::verdict;

/// Evidence command run ON the box through SSM; no quote characters inside.
const EVIDENCE_CMD: &str = "{ systemctl status docker --no-pager; docker image ls -a --no-trunc; tail -n 200 /var/log/forjar-spot-seat.log; tail -n 200 /var/log/cloud-init-output.log; } 2>&1";

fn live_ids(s: &SpotSeat) -> String {
    format!(
        "aws ec2 describe-instances --region {} --filters Name=tag:Owner,Values={} Name=instance-state-name,Values=pending,running --query 'Reservations[].Instances[].InstanceId' --output text",
        sh_squote(&s.region),
        sh_squote(&s.owner)
    )
}

fn day() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400)
}

fn token(s: &SpotSeat) -> String {
    let h = crate::tripwire::hasher::hash_string(&format!(
        "{}|{}|{}|{}",
        s.owner,
        s.instance_types.join(","),
        s.az,
        day()
    ));
    format!("fj-{}", &h["blake3:".len()..][..32])
}

fn launch(s: &SpotSeat) -> Result<String, ForjarError> {
    let ami = s
        .ami
        .as_deref()
        .ok_or_else(|| ForjarError::validation("launch needs `spot_ami`"))?;
    let tags = format!(
        "ResourceType=instance,Tags=[{{Key=Owner,Value={o}}},{{Key=Name,Value=seat-{o}}},{{Key=TtlMin,Value={t}}},{{Key=Price,Value={p}}}]",
        o = s.owner,
        t = s.ttl_min,
        p = s.price_usd_per_hour
    );
    let sreq = format!(
        "ResourceType=spot-instances-request,Tags=[{{Key=Owner,Value={}}}]",
        s.owner
    );
    let types: Vec<String> = s.instance_types.iter().map(|t| sh_squote(t)).collect();
    Ok(format!(
        "set -u\nR={region}\nCT={ct}\nMAXB={maxb}\nCAP={cap}\nPRICE={price}\n\
         ids=\"$({live})\" || {{ echo 'live-box-read-failed (not measured)' >&2; exit 1; }}\n\
         adopt=\"$(aws ec2 describe-instances --region \"$R\" --filters \"Name=client-token,Values=$CT-*\" Name=instance-state-name,Values=pending,running --query 'Reservations[0].Instances[0].InstanceId' --output text 2>/dev/null)\" || adopt=None\n\
         case \"$adopt\" in i-*) echo \"adopted $adopt\"; exit 0;; esac\n\
         n=0; for _ in $ids; do n=$((n+1)); done\n\
         [ $((n+1)) -le \"$MAXB\" ] || {{ echo \"REFUSED: cap: $((n+1)) boxes exceeds spot_max_boxes $MAXB\" >&2; exit 3; }}\n\
         raw=\"$(aws ec2 describe-instances --region \"$R\" --filters Name=tag:Owner,Values={owner} Name=instance-state-name,Values=pending,running --query 'Reservations[].Instances[].Tags[?Key==`Price`].Value' --output text)\" || {{ echo 'price-read-failed (not measured)' >&2; exit 1; }}\n\
         spent=\"$(printf '%s\\n' \"$raw\" | tr '\\t' '\\n' | awk 'BEGIN{{s=0}} {{s+=$1}} END{{print s}}')\" || {{ echo 'price-sum-failed (not measured)' >&2; exit 1; }}\n\
         awk -v a=\"$spent\" -v p=\"$PRICE\" -v c=\"$CAP\" 'BEGIN{{exit !(a+p<=c)}}' || {{ echo \"REFUSED: cap: $spent + $PRICE USD/h exceeds spot_max_usd_per_hour $CAP\" >&2; exit 3; }}\n\
         err=\"$(mktemp)\"\n\
         for t in {types}; do\n  \
           if id=\"$(aws ec2 run-instances --region \"$R\" --client-token \"$CT-$t\" --image-id {ami} --instance-type \"$t\" --subnet-id {subnet} --count 1 \\\n    \
             --instance-market-options \"MarketType=spot,SpotOptions={{MaxPrice=$PRICE,SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}\" \\\n    \
             --instance-initiated-shutdown-behavior terminate --tag-specifications {tags} {sreq} --query 'Instances[0].InstanceId' --output text 2>\"$err\")\"; then echo \"launched $id\"; exit 0; fi\n  \
           if grep -q 'InsufficientInstanceCapacity\\|SpotMaxPriceTooLow\\|MaxSpotInstanceCountExceeded' \"$err\"; then echo \"no-capacity:$t\" >&2; continue; fi\n  \
           echo \"launch-failed:$t: $(tail -n 3 \"$err\" | tr '\\n' ' ')\" >&2; exit 1\n\
         done\n\
         echo 'REFUSED: no capacity for any instance type' >&2; exit 75\n",
        region = sh_squote(&s.region),
        ct = sh_squote(&token(s)),
        maxb = s.max_boxes,
        cap = s.max_usd_per_hour,
        price = s.price_usd_per_hour,
        live = live_ids(s),
        owner = sh_squote(&s.owner),
        types = types.join(" "),
        ami = sh_squote(ami),
        subnet = sh_squote(&s.subnet),
        tags = sh_squote(&tags),
        sreq = sh_squote(&sreq),
    ))
}

fn gh_runners(s: &SpotSeat) -> String {
    format!(
        "gh api --paginate {} --jq '.runners[]|[.id,.name,.status,.busy]|@tsv'",
        sh_squote(&format!(
            "orgs/{}/actions/runners?per_page=100",
            s.runner_org
        ))
    )
}

/// Deregister stale offline seats of this owner, then mint one JIT per missing seat.
fn register(s: &SpotSeat) -> String {
    let org = sh_squote(&s.runner_org);
    let labels: String = s
        .runner_labels
        .iter()
        .map(|l| format!(" -f {}", sh_squote(&format!("labels[]={l}"))))
        .collect();
    format!(
        "set -u\numask 077\nORG={org}\nGROUP={group}\nOWNER={owner}\nN={n}\nJITDIR={jit}\n\
         list=\"$({list})\" || {{ echo 'runner-list-failed (not measured)' >&2; exit 1; }}\n\
         printf '%s\\n' \"$list\" | while IFS=\"$(printf '\\t')\" read -r id name status busy; do\n  \
           case \"$name\" in \"seat-$OWNER-\"*) ;; *) continue;; esac\n  \
           if [ \"$status\" = offline ] && [ \"$busy\" = false ]; then gh api -X DELETE \"orgs/$ORG/actions/runners/$id\" >/dev/null || exit 1; echo \"deregistered:$name\"; fi\n\
         done || exit 1\n\
         case \"$GROUP\" in *[!0-9]*) gid=\"$(gh api \"orgs/$ORG/actions/runner-groups\" --jq \".runner_groups[]|select(.name==\\\"$GROUP\\\")|.id\")\" || exit 1;; *) gid=\"$GROUP\";; esac\n\
         [ -n \"$gid\" ] || {{ echo \"runner-group-not-found:$GROUP\" >&2; exit 1; }}\n\
         mkdir -p \"$JITDIR\"; i=1\n\
         while [ \"$i\" -le \"$N\" ]; do\n  \
           name=\"$(printf 'seat-%s-%02d' \"$OWNER\" \"$i\")\"\n  \
           if printf '%s\\n' \"$list\" | awk -v n=\"$name\" '$2==n && $3==\"online\"{{f=1}} END{{exit !f}}'; then echo \"registered:$name\"; i=$((i+1)); continue; fi\n  \
           gh api -X POST \"orgs/$ORG/actions/runners/generate-jitconfig\" -f \"name=$name\" -F \"runner_group_id=$gid\"{labels} -f 'work_folder=_work' --jq .encoded_jit_config >\"$JITDIR/$name.jit\" || {{ echo \"jit-mint-failed:$name\" >&2; exit 1; }}\n  \
           echo \"jit-minted:$name\"; i=$((i+1))\n\
         done\n",
        group = sh_squote(&s.runner_group),
        owner = sh_squote(&s.owner),
        n = s.seats,
        jit = sh_squote(&jit_dir(s)),
        list = gh_runners(s),
    )
}

fn teardown(s: &SpotSeat) -> Result<String, ForjarError> {
    let dest = s
        .evidence_dest
        .as_deref()
        .filter(|d| {
            d.strip_prefix("s3://").is_some_and(|r| {
                !r.is_empty()
                    && r.chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '/'))
            })
        })
        .ok_or_else(|| {
            ForjarError::validation("teardown needs `spot_evidence_dest: s3://bucket/prefix`")
        })?;
    let params =
        format!("commands=[\"{{ {EVIDENCE_CMD} ; }} | aws s3 cp - {dest}/ID/evidence.txt\"]");
    Ok(format!(
        "set -u\nR={region}\nOWNER={owner}\n\
         ids=\"$({live})\" || {{ echo 'live-box-read-failed (not measured)' >&2; exit 1; }}\n\
         for id in $ids; do\n  \
           cmd=\"$(aws ssm send-command --region \"$R\" --instance-ids \"$id\" --document-name AWS-RunShellScript --parameters \"$(printf '%s' {params} | sed \"s/ID/$id/\")\" --query 'Command.CommandId' --output text)\" || {{ echo \"evidence-ship-failed:$id; nothing terminated\" >&2; exit 1; }}\n  \
           aws ssm wait command-executed --region \"$R\" --command-id \"$cmd\" --instance-id \"$id\" || {{ echo \"evidence-ship-failed:$id; nothing terminated\" >&2; exit 1; }}\n  \
           echo \"evidence-shipped:$id\"\n\
         done\n\
         reqs=\"$(aws ec2 describe-spot-instance-requests --region \"$R\" --filters Name=tag:Owner,Values=\"$OWNER\" Name=state,Values=open,active --query 'SpotInstanceRequests[].SpotInstanceRequestId' --output text)\" || {{ echo 'spot-request-read-failed' >&2; exit 1; }}\n\
         [ -z \"$reqs\" ] || {{ aws ec2 cancel-spot-instance-requests --region \"$R\" --spot-instance-request-ids $reqs >/dev/null || {{ echo 'spot-cancel-failed' >&2; exit 1; }}; echo \"cancelled:$reqs\"; }}\n\
         [ -z \"$ids\" ] || {{ aws ec2 terminate-instances --region \"$R\" --instance-ids $ids >/dev/null || {{ echo 'terminate-failed' >&2; exit 1; }}; aws ec2 wait instance-terminated --region \"$R\" --instance-ids $ids || {{ echo 'terminate-wait-failed' >&2; exit 1; }}; echo \"terminated:$ids\"; }}\n\
         left=\"$({live})\" || {{ echo 'read-back-failed' >&2; exit 1; }}\n\
         n=0; for _ in $left; do n=$((n+1)); done\n\
         [ \"$n\" -eq 0 ] || {{ echo \"state shows $n boxes after teardown, want 0\" >&2; exit 1; }}\n\
         echo 'state shows 0'\n",
        region = sh_squote(&s.region),
        owner = sh_squote(&s.owner),
        live = live_ids(s),
        params = sh_squote(&params),
    ))
}

pub(super) fn apply(s: &SpotSeat) -> Result<String, ForjarError> {
    match s.phase {
        Phase::Launch => launch(s),
        Phase::Register => Ok(register(s)),
        Phase::Teardown => teardown(s),
        Phase::ImageLoad | Phase::SeatStart => {
            Err(ForjarError::other("on-box phase routed to controller"))
        }
    }
}

pub(super) fn check(s: &SpotSeat) -> String {
    let n = format!("n=0; for _ in $({}); do n=$((n+1)); done\n", live_ids(s));
    let a = match s.phase {
        Phase::Teardown => verdict::assert_that("[ \"$n\" -eq 0 ]", "spot-boxes-none", "spot-boxes-remain"),
        Phase::Register => {
            return format!(
                "online=\"$({} | awk -v p={} '$2 ~ \"^\"p && $3==\"online\"' | wc -l | tr -d ' ')\" || online=0\nn=\"$online\"\n{}",
                gh_runners(s),
                sh_squote(&format!("seat-{}-", s.owner)),
                verdict::check_script_from(&[verdict::assert_that(
                    &format!("[ \"$n\" -ge {} ]", s.seats),
                    "spot-seats-registered",
                    "spot-seats-unregistered"
                )])
            )
        }
        _ => verdict::assert_that("[ \"$n\" -ge 1 ]", "spot-box-present", "spot-box-absent"),
    };
    format!("{n}{}", verdict::check_script_from(&[a]))
}

pub(super) fn state_query(s: &SpotSeat) -> String {
    match s.phase {
        Phase::Register => format!("{} | awk -v p={} 'BEGIN{{n=0}} $2 ~ \"^\"p && $3==\"online\"{{n++}} END{{print \"seats_online=\" n}}'\n", gh_runners(s), sh_squote(&format!("seat-{}-", s.owner))),
        _ => format!("n=0; for _ in $({}); do n=$((n+1)); done; echo \"boxes=$n\"\n", live_ids(s)),
    }
}
