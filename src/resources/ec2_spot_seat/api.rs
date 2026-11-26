//! The outside world of an `ec2_spot_seat`, behind traits so tests use fakes and
//! nothing in `cargo test` can reach AWS or GitHub.

use crate::core::error::ForjarError;

/// One live (pending or running) box carrying the owner tag.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveBox {
    pub id: String,
    pub price_usd_per_hour: f64,
}

/// One `run-instances` request: spot market, one-time, terminate on shutdown.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchRequest {
    pub instance_type: String,
    pub az: String,
    pub subnet: String,
    pub ami: Option<String>,
    pub owner: String,
    pub ttl_min: u32,
    pub price_usd_per_hour: f64,
    /// Same token on a retry, so a retried launch cannot start a second box.
    pub client_token: String,
}

/// Why a launch failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    /// No capacity for this type here: the next candidate may work.
    Capacity(String),
    /// Anything else: stop.
    Other(String),
}

/// EC2 calls the resource needs.
pub trait Ec2Api {
    /// Live boxes tagged `Owner=<owner>`. A failed read is `Err`, never "none".
    fn live_boxes(&self, owner: &str) -> Result<Vec<LiveBox>, ForjarError>;
    /// Launch one spot box; returns its instance id.
    fn run_spot(&self, req: &LaunchRequest) -> Result<String, LaunchError>;
    /// Open spot requests of the owner (any state that can still fulfil).
    fn open_spot_requests(&self, owner: &str) -> Result<Vec<String>, ForjarError>;
    /// Cancel spot requests.
    fn cancel_spot_requests(&self, ids: &[String]) -> Result<(), ForjarError>;
    /// Terminate instances.
    fn terminate(&self, ids: &[String]) -> Result<(), ForjarError>;
}

/// Ships a box's evidence (unit status, `docker image ls -a`, entrypoint log) off-box.
pub trait EvidenceSink {
    /// Ship the evidence of box `id` to `dest`; `Err` stops the teardown.
    fn ship(&self, id: &str, dest: &str) -> Result<(), ForjarError>;
}

/// One registered org runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runner {
    pub id: u64,
    pub name: String,
    pub online: bool,
    pub busy: bool,
}

/// GitHub org-runner calls the resource needs.
pub trait RunnerApi {
    /// Every runner of the org. A failed read is `Err` (not measured).
    fn list_runners(&self, org: &str) -> Result<Vec<Runner>, ForjarError>;
    /// Remove one registration.
    fn delete_runner(&self, org: &str, id: u64) -> Result<(), ForjarError>;
    /// Mint one JIT config for a seat in `group` with `labels`.
    fn mint_jit(
        &self,
        org: &str,
        group: &str,
        name: &str,
        labels: &[String],
    ) -> Result<String, ForjarError>;
}
