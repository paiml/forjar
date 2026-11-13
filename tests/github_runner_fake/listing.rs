//! forjar#1000: how the fake answers `GET .../actions/runners`.
//!
//! `Normal` lists the scripted seats. The other two model a listing forjar
//! cannot read to the end, which must be reported as an error, never as
//! "absent": a reply with no `runners` array, and a listing that is still full
//! (100 seats) on every page.

use super::{runner_json, runners_page, FakeGithub, ListedRunner, State};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Listing {
    #[default]
    Normal,
    NoRunnersArray,
    FullOnEveryPage,
}

impl FakeGithub {
    pub fn set_listing(&self, listing: Listing) {
        self.state.lock().unwrap().listing = listing;
    }
}

pub(super) fn body(st: &State, page_two: bool) -> serde_json::Value {
    match st.listing {
        Listing::Normal => runners_page(&st.runners.iter().collect::<Vec<_>>(), page_two),
        Listing::NoRunnersArray => serde_json::json!({"total_count": 0}),
        Listing::FullOnEveryPage => {
            let page: Vec<_> = (0..100)
                .map(|i| {
                    runner_json(&ListedRunner {
                        name: format!("other-{i}"),
                        custom_labels: Vec::new(),
                        group: "Default".to_string(),
                    })
                })
                .collect();
            serde_json::json!({"total_count": 100_000, "runners": page})
        }
    }
}
