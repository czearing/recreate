//! Aiming at a control and completing the activation, kept apart from restoring the page so
//! the point where the trigger is focused but not yet pressed has an owner.

use super::interactions_scripts::{Candidate, PREFLIGHT};
use crate::{
    cdp::Cdp,
    interactions_input::{aim_matching, press, submit_text_matching},
};
use anyhow::Result;
/// An activation split at the point where the trigger has been focused but not yet pressed.
///
/// The evidence baseline is measured here, by the aim itself, rather than by the caller. A
/// probe must focus what it points at, and a focus ring is a style change, so a baseline taken
/// before the aim would differ from the result for every focusable control on the page and
/// report an interaction that the page does not have. Producing the baseline from the aim
/// leaves no way to measure it any earlier.
pub(super) struct Aimed {
    action: Action,
    baseline: serde_json::Value,
}

enum Action {
    Press(String, (f64, f64)),
    Submitted,
    Unreachable,
}

pub(super) async fn aim(cdp: &mut Cdp, candidate: &Candidate) -> Result<Aimed> {
    let action = if candidate.uses_text_entry() {
        let submitted = submit_text_matching(
            cdp,
            &candidate.path,
            &candidate.tag,
            &candidate.label,
            Some(candidate.occurrence),
        )
        .await?;
        if submitted {
            Action::Submitted
        } else {
            Action::Unreachable
        }
    } else {
        match aim_matching(
            cdp,
            &candidate.path,
            &candidate.tag,
            &candidate.label,
            Some(candidate.occurrence),
            false,
        )
        .await?
        {
            Some(position) => Action::Press(candidate.path.clone(), position),
            None => Action::Unreachable,
        }
    };
    let baseline = cdp.evaluate(PREFLIGHT).await?;
    Ok(Aimed { action, baseline })
}

/// Completes an aimed activation, reporting the baseline it was measured against, or `None`
/// when the trigger could not be reached.
pub(super) async fn fire(cdp: &mut Cdp, aimed: Aimed) -> Result<Option<serde_json::Value>> {
    match aimed.action {
        Action::Press(path, position) => press(cdp, &path, position).await?,
        Action::Submitted => {}
        Action::Unreachable => return Ok(None),
    }
    Ok(Some(aimed.baseline))
}

pub(super) async fn activate(cdp: &mut Cdp, candidate: &Candidate) -> Result<bool> {
    let aimed = aim(cdp, candidate).await?;
    Ok(fire(cdp, aimed).await?.is_some())
}
