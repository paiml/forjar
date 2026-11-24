//! Production [`Ec2Api`]: shells out to the `aws` CLI through an injected [`CliRunner`].
//!
//! Spot only: [`run_instances_args`] refuses any market but `spot`, so there is no
//! on-demand path. Every call passes `--region`. Tests use a fake runner; only
//! [`ProcessRunner`] spawns a process.

use super::api::{Ec2Api, LaunchError, LaunchRequest, LiveBox};
use serde_json::{json, Value};
use std::process::Command;

/// Tag carrying the hourly price a box was launched at (read back by `live_boxes`).
pub const PRICE_TAG: &str = "PriceUsdPerHour";

/// Runs one CLI invocation and returns stdout; `Err` carries stderr or the spawn error.
pub trait CliRunner {
    fn run(&self, args: &[&str]) -> Result<String, String>;
}

/// Real runner: spawns the `aws` binary.
pub struct ProcessRunner;

impl CliRunner for ProcessRunner {
    fn run(&self, args: &[&str]) -> Result<String, String> {
        let out = Command::new("aws")
            .args(args)
            .output()
            .map_err(|e| format!("cannot run aws: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

/// `Ec2Api` over the aws CLI.
pub struct AwsCli<R: CliRunner> {
    runner: R,
    region: String,
}

impl AwsCli<ProcessRunner> {
    /// The production adapter for `region`.
    #[must_use]
    pub fn new(region: &str) -> Self {
        Self::with_runner(ProcessRunner, region)
    }
}

impl<R: CliRunner> AwsCli<R> {
    /// An adapter over any runner (tests pass a fake).
    #[must_use]
    pub fn with_runner(runner: R, region: &str) -> Self {
        Self {
            runner,
            region: region.to_string(),
        }
    }

    fn call(&self, args: &[String]) -> Result<Value, String> {
        let mut full: Vec<&str> = args.iter().map(String::as_str).collect();
        full.extend(["--region", &self.region, "--output", "json"]);
        let out = self.runner.run(&full)?;
        if out.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&out).map_err(|e| format!("unparseable aws output: {e}"))
    }
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

/// Build the `run-instances` argument list. Refuses any market but `spot`.
///
/// # Errors
///
/// `Err` for a non-spot market or a request without an image id.
pub fn run_instances_args(req: &LaunchRequest, market: &str) -> Result<Vec<String>, String> {
    if market != "spot" {
        return Err(format!("refused: market `{market}`; only spot is allowed"));
    }
    let ami = req
        .ami
        .as_deref()
        .ok_or("run-instances needs an image id (`spot_ami`)")?;
    let tags = json!([
        {"Key": "Owner", "Value": req.owner},
        {"Key": "TtlMin", "Value": req.ttl_min.to_string()},
        {"Key": PRICE_TAG, "Value": req.price_usd_per_hour.to_string()},
    ]);
    let specs = json!([
        {"ResourceType": "instance", "Tags": tags},
        {"ResourceType": "spot-instances-request", "Tags": tags},
    ]);
    let market_opts = json!({
        "MarketType": "spot",
        "SpotOptions": {
            "MaxPrice": req.price_usd_per_hour.to_string(),
            "SpotInstanceType": "one-time",
            "InstanceInterruptionBehavior": "terminate",
        },
    });
    let mut a = strs(&["ec2", "run-instances", "--count", "1"]);
    for (k, v) in [
        ("--image-id", ami.to_string()),
        ("--instance-type", req.instance_type.clone()),
        ("--subnet-id", req.subnet.clone()),
        ("--placement", format!("AvailabilityZone={}", req.az)),
        ("--client-token", req.client_token.clone()),
        ("--instance-initiated-shutdown-behavior", "terminate".into()),
        ("--instance-market-options", market_opts.to_string()),
        ("--tag-specifications", specs.to_string()),
    ] {
        a.push(k.into());
        a.push(v);
    }
    Ok(a)
}

fn is_capacity(msg: &str) -> bool {
    [
        "InsufficientInstanceCapacity",
        "InsufficientCapacity",
        "SpotMaxPriceTooLow",
        "MaxSpotInstanceCountExceeded",
        "capacity-not-available",
        "capacity-oversubscribed",
        "price-too-low",
    ]
    .iter()
    .any(|m| msg.contains(m))
}

fn tag<'a>(inst: &'a Value, key: &str) -> Option<&'a str> {
    inst["Tags"]
        .as_array()?
        .iter()
        .find(|t| t["Key"] == key)
        .and_then(|t| t["Value"].as_str())
}

fn owner_filter(owner: &str) -> String {
    format!("Name=tag:Owner,Values={owner}")
}

impl<R: CliRunner> Ec2Api for AwsCli<R> {
    fn live_boxes(&self, owner: &str) -> Result<Vec<LiveBox>, String> {
        let v = self.call(&[
            "ec2".into(),
            "describe-instances".into(),
            "--filters".into(),
            owner_filter(owner),
            "Name=instance-state-name,Values=pending,running".into(),
        ])?;
        let res = v["Reservations"]
            .as_array()
            .ok_or("describe-instances: no Reservations array")?;
        let mut out = Vec::new();
        for inst in res
            .iter()
            .filter_map(|r| r["Instances"].as_array())
            .flatten()
        {
            let id = inst["InstanceId"]
                .as_str()
                .ok_or("describe-instances: instance without InstanceId")?;
            // An unknown price must not read as zero cost: fail the read instead.
            let price = tag(inst, PRICE_TAG)
                .and_then(|p| p.parse::<f64>().ok())
                .filter(|p| p.is_finite() && *p >= 0.0)
                .ok_or_else(|| format!("box {id} has no readable {PRICE_TAG} tag"))?;
            out.push(LiveBox {
                id: id.to_string(),
                price_usd_per_hour: price,
            });
        }
        Ok(out)
    }

    fn run_spot(&self, req: &LaunchRequest) -> Result<String, LaunchError> {
        let args = run_instances_args(req, "spot").map_err(LaunchError::Other)?;
        let v = self.call(&args).map_err(|e| {
            if is_capacity(&e) {
                LaunchError::Capacity(e)
            } else {
                LaunchError::Other(e)
            }
        })?;
        v["Instances"][0]["InstanceId"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| LaunchError::Other("run-instances returned no instance id".into()))
    }

    fn open_spot_requests(&self, owner: &str) -> Result<Vec<String>, String> {
        let v = self.call(&[
            "ec2".into(),
            "describe-spot-instance-requests".into(),
            "--filters".into(),
            owner_filter(owner),
            "Name=state,Values=open".into(),
        ])?;
        let reqs = v["SpotInstanceRequests"]
            .as_array()
            .ok_or("describe-spot-instance-requests: no SpotInstanceRequests array")?;
        reqs.iter()
            .map(|r| {
                r["SpotInstanceRequestId"]
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| "spot request without an id".to_string())
            })
            .collect()
    }

    fn cancel_spot_requests(&self, ids: &[String]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut a = strs(&[
            "ec2",
            "cancel-spot-instance-requests",
            "--spot-instance-request-ids",
        ]);
        a.extend(ids.iter().cloned());
        self.call(&a).map(|_| ())
    }

    fn terminate(&self, ids: &[String]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut a = strs(&["ec2", "terminate-instances", "--instance-ids"]);
        a.extend(ids.iter().cloned());
        self.call(&a).map(|_| ())
    }
}
