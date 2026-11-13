//! forjar#1000: a runner listing forjar cannot read to the end is an error,
//! never "absent".
//!
//! Reading a listing with no `runners` array as empty, or giving up after 50
//! full pages and calling the seat unlisted, would make apply request a
//! registration token and register a second runner under a name GitHub may
//! already hold, and would make check report the seat missing on a reply it
//! never measured. Both apply and check must fail, say why, and request no
//! token and run nothing on the seat. Real `forjar` binary, local transport,
//! local fake of the GitHub API.
//!
//! Usage: cargo test --test falsification_github_runner_listing

mod github_runner_fake;

use github_runner_fake::{Listing, Rig};

const LABELS: &str = "[self-hosted, linux, x64, gate]";
const LISTING: &str = "/actions/runners";

/// apply then check against a fake that answers the listing with `listing`;
/// both must fail naming `why`, with no token requested and no seat script.
fn unmeasured(listing: Listing, why: &str) -> Rig {
    let rig = Rig::new(LABELS, true);
    rig.fake.set_listing(listing);
    for (verb, out) in [
        (
            "apply",
            rig.apply(&rig.state_dir("st-apply").display().to_string()),
        ),
        (
            "check",
            rig.check(&rig.state_dir("st-check").display().to_string()),
        ),
    ] {
        assert!(
            !out.ok,
            "{verb} passed on an unreadable listing: {}",
            out.text
        );
        assert!(
            out.text.contains(why),
            "{verb} must say {why:?}: {}",
            out.text
        );
    }
    assert_eq!(rig.fake.token_requests(), 0, "no registration token");
    assert_eq!(rig.seat_commands(), 0, "nothing ran on the seat");
    rig
}

#[test]
fn listing_without_a_runners_array_is_an_error() {
    let rig = unmeasured(Listing::NoRunnersArray, "no `runners` array");
    assert_eq!(
        rig.fake.count("GET", LISTING),
        2,
        "one listing read per verb"
    );
}

#[test]
fn listing_over_fifty_pages_is_an_error() {
    let rig = unmeasured(Listing::FullOnEveryPage, "more than 50 pages");
    assert_eq!(
        rig.fake.count("GET", LISTING),
        100,
        "50 pages per verb, then stop"
    );
}
