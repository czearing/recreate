//! Watching a page for the recurring attribute changes that become sequences.
//!
//! This used to be a Rust loop polling the page every 250ms behind a four-second floor and
//! a twelve-second window of its own — a second, drifted copy of "has this page stopped
//! producing new information?", with a constant standing in for every part of the answer.
//! Every capture of every page paid the floor, whether or not the page had any attribute
//! behaviour at all, and a page with a looping animation paid the whole window twice over.
//!
//! The question already has an owner. `lifecycle_settle_script` decides when a recorder may
//! stop, from the longest gap in change the page has already recovered from and from direct
//! evidence that something is still to come. Here that evidence is a change group whose
//! values have not yet proven a repeating cycle. Inlining the one predicate rather than
//! restating it in Rust is what keeps the two from drifting apart again, and moving the loop
//! into the page replaces the poll interval with an animation frame.

use anyhow::Result;

const OBSERVE: &str = include_str!("dynamic_observe.js");

/// The page script that resolves once no unfinished sequence remains and the page has been
/// quiet for longer than any gap in change it has already recovered from.
pub(super) fn source() -> String {
    OBSERVE.replace(
        "__LIFECYCLE_SETTLE__",
        crate::lifecycle_settle_script::SOURCE,
    )
}

/// Watches the page until its attribute behaviour has finished arriving.
pub(super) async fn observe(cdp: &mut crate::cdp::Cdp) -> Result<()> {
    cdp.evaluate(&source()).await?;
    Ok(())
}

#[cfg(test)]
#[path = "dynamic_tests.rs"]
mod tests;
