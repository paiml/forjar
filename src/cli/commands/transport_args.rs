//! Arguments for the derived transport verbs (`serve`).
//!
//! These verbs expose the [`crate::verb`] registry over a wire protocol. They
//! declare no per-operation flags of their own, because the operations are not
//! theirs: every verb they serve comes from the same clap tree this file is
//! part of.

/// CLI arguments for the `serve` command.
#[derive(clap::Args, Debug)]
pub struct ServeArgs {
    /// Port to listen on
    #[arg(long, default_value = "8737")]
    pub port: u16,

    /// Address to bind (use 0.0.0.0 to accept non-local connections)
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Refuse verbs that can change the world; serve only read-only ones
    #[arg(long)]
    pub read_only: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct Harness {
        #[command(flatten)]
        args: ServeArgs,
    }

    #[test]
    fn defaults_bind_loopback_only() {
        // A default of 0.0.0.0 would put an unauthenticated verb executor on
        // every interface the moment someone typed `forjar serve`.
        let h = Harness::try_parse_from(["x"]).unwrap();
        assert_eq!(h.args.host, "127.0.0.1");
        assert_eq!(h.args.port, 8737);
        assert!(!h.args.read_only);
    }

    #[test]
    fn port_and_host_are_overridable() {
        let h = Harness::try_parse_from(["x", "--port", "9000", "--host", "0.0.0.0"]).unwrap();
        assert_eq!(h.args.port, 9000);
        assert_eq!(h.args.host, "0.0.0.0");
    }

    #[test]
    fn a_port_outside_u16_is_rejected_by_the_parser() {
        assert!(Harness::try_parse_from(["x", "--port", "70000"]).is_err());
        assert!(Harness::try_parse_from(["x", "--port", "-1"]).is_err());
    }

    #[test]
    fn read_only_is_a_flag() {
        let h = Harness::try_parse_from(["x", "--read-only"]).unwrap();
        assert!(h.args.read_only);
    }
}
