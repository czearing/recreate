use super::{
    interactions_activate::activate,
    interactions_runtime::{restore, settle},
    interactions_scripts::{ACTION_SCOPE, Candidate, TAKE_SCOPE},
};
use crate::{capture, cdp::Cdp, model::PageState};
use anyhow::Result;

/// Records every node the page touches while one action runs, so the resulting state can be
/// narrowed to what the action actually caused rather than everything that happened to differ.
pub(super) async fn begin_scope(cdp: &mut Cdp, trigger: &str) -> Result<()> {
    cdp.evaluate(&format!(
        "({ACTION_SCOPE})({})",
        serde_json::to_string(trigger)?
    ))
    .await?;
    Ok(())
}

pub(super) async fn take_scope(cdp: &mut Cdp) -> Result<Vec<String>> {
    Ok(serde_json::from_value(cdp.evaluate(TAKE_SCOPE).await?)?)
}

pub(super) async fn reach(
    cdp: &mut Cdp,
    baseline: &PageState,
    prefix: &[Candidate],
) -> Result<Option<(PageState, PageState)>> {
    let fresh = restore(cdp, baseline, false).await?;
    for action in prefix {
        if !activate(cdp, action).await? {
            return Ok(None);
        }
        let _ = settle(cdp, action.uses_text_entry()).await?;
    }
    let reached = capture::read_interaction_state(cdp, baseline.viewport.clone()).await?;
    Ok(Some((fresh, reached)))
}
