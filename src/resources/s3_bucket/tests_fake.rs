//! `s3_bucket` falsifiers. A stateful fake `aws` stands in for S3: `put-*` writes what
//! the later `get-*` returns, every `--query` is compared with the expected expression
//! (a typo answers BADQUERY, which no assertion accepts), and every call is logged.

use super::*;
use crate::core::types::{ResourceType, S3Spec};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub(super) const BUCKET: &str = "paiml-metrics-preview";

pub(super) fn res() -> Resource {
    Resource {
        resource_type: ResourceType::S3Bucket,
        s3: S3Spec {
            bucket: Some(BUCKET.into()),
            region: Some("us-east-1".into()),
            private: Some(true),
            sse: Some("AES256".into()),
            tls_only: Some(true),
            versioning: Some(true),
            noncurrent_expire_days: Some(30),
        },
        ..Default::default()
    }
}

const STUB: &str = r#"#!/bin/sh
d="$S3_STUB_DIR"
echo "$*" >> "$d/log"
[ "$1" = s3api ] || exit 64
op="$2"; shift 2
q=''; pol=''; prev=''
for a in "$@"; do
  [ "$prev" = --query ] && q="$a"
  [ "$prev" = --policy ] && pol="$a"
  prev="$a"
done
want() { [ "$q" = "$1" ] || { echo BADQUERY; exit 0; }; }
fail() { echo "An error occurred ($1) when calling the $op operation: $2" >&2; exit 254; }
case "$op" in
head-bucket)
  [ -e "$d/head.err" ] && { cat "$d/head.err" >&2; exit 254; }
  [ -e "$d/exists" ] || fail 404 'Not Found'; exit 0;;
create-bucket) touch "$d/exists"; exit 0;;
get-bucket-encryption)
  want 'ServerSideEncryptionConfiguration.Rules[0].ApplyServerSideEncryptionByDefault.SSEAlgorithm'
  [ -e "$d/enc" ] || fail 404 ServerSideEncryptionConfigurationNotFoundError; cat "$d/enc";;
put-bucket-encryption) printf 'AES256\n' > "$d/enc";;
get-public-access-block)
  want 'PublicAccessBlockConfiguration.[BlockPublicAcls,IgnorePublicAcls,BlockPublicPolicy,RestrictPublicBuckets]'
  [ -e "$d/pab" ] || fail 404 NoSuchPublicAccessBlockConfiguration; cat "$d/pab";;
put-public-access-block) printf 'True\tTrue\tTrue\tTrue\n' > "$d/pab";;
get-bucket-versioning)
  want Status
  [ -e "$d/ver" ] && cat "$d/ver" || echo None;;
put-bucket-versioning) printf 'Enabled\n' > "$d/ver";;
get-bucket-lifecycle-configuration)
  want 'Rules[?ID==`forjar-noncurrent-expire`]|[0].[Status,NoncurrentVersionExpiration.NoncurrentDays]'
  [ -e "$d/lc" ] || fail 404 NoSuchLifecycleConfiguration; cat "$d/lc";;
put-bucket-lifecycle-configuration)
  days=$(printf '%s' "$*" | sed -n 's/.*"NoncurrentDays":\([0-9]*\).*/\1/p')
  printf 'Enabled\t%s\n' "$days" > "$d/lc";;
get-bucket-policy)
  want Policy
  [ -e "$d/pol" ] || fail 404 NoSuchBucketPolicy; cat "$d/pol";;
put-bucket-policy) printf '%s' "$pol" | sed 's/,/,\n  /g' > "$d/pol";;
*) exit 65;;
esac
exit 0
"#;

pub(super) struct Fake {
    dir: tempfile::TempDir,
}

impl Fake {
    pub(super) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("bin")).unwrap();
        let aws = dir.path().join("bin/aws");
        std::fs::write(&aws, STUB).unwrap();
        std::fs::set_permissions(&aws, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self { dir }
    }
    pub(super) fn path(&self) -> &Path {
        self.dir.path()
    }
    pub(super) fn set(&self, name: &str, body: &str) {
        std::fs::write(self.path().join(name), body).unwrap();
    }
    pub(super) fn unset(&self, name: &str) {
        let _ = std::fs::remove_file(self.path().join(name));
    }
    /// A fully converged bucket, produced by running the apply script on an empty one.
    pub(super) fn converged() -> Self {
        let f = Self::new();
        let (rc, _, err) = f.run(&apply_script(&res()));
        assert_eq!(rc, 0, "apply on an empty fake failed: {err}");
        f
    }
    pub(super) fn run(&self, script: &str) -> (i32, String, String) {
        let path = format!(
            "{}:{}",
            self.path().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(script)
            .env("PATH", path)
            .env("S3_STUB_DIR", self.path())
            .output()
            .unwrap();
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
    pub(super) fn log(&self) -> String {
        std::fs::read_to_string(self.path().join("log")).unwrap_or_default()
    }
    pub(super) fn clear_log(&self) {
        self.unset("log");
    }
    pub(super) fn file(&self, name: &str) -> PathBuf {
        self.path().join(name)
    }
}

pub(super) fn mutating(log: &str) -> Vec<&str> {
    log.lines()
        .filter(|l| l.contains(" create-") || l.contains(" put-"))
        .collect()
}
