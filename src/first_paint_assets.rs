//! The single owner of "what content did the loading phase need that the settled page does
//! not?".
//!
//! A phase's own images and fonts are reachable only from nodes that no longer exist, so the
//! settled capture never names them and the download pass that runs with it never sees them.
//! Left there, the generator holds URLs it cannot resolve and the replay renders with holes.
//! The reading taken at first paint deliberately downloads nothing — it must not put requests
//! in flight that the settle gate would then wait on — so the shortfall is made up here,
//! once, against the exact set the settled page turns out not to carry.

use crate::{asset_script, cdp::Cdp, model::PageState};
use anyhow::Result;
use std::collections::BTreeMap;

/// Folds the phase's asset table into the settled one and fetches whatever is still only a
/// URL. Content already held wins, because the settled page read it from a live element.
pub async fn merge(cdp: &mut Cdp, state: &mut PageState, first: PageState) -> Result<()> {
    for (url, data) in first.asset_data {
        state.asset_data.entry(url).or_insert(data);
    }
    let missing: Vec<String> = first
        .asset_urls
        .into_iter()
        .filter(|url| !state.asset_urls.contains(url))
        .collect();
    state.asset_urls.extend(missing.iter().cloned());
    let wanted: Vec<String> = missing
        .into_iter()
        .filter(|url| !state.asset_data.contains_key(url))
        .collect();
    if wanted.is_empty() {
        return Ok(());
    }
    let raw = cdp
        .evaluate(&asset_script::download_source(&wanted))
        .await?;
    if let Some(text) = raw.as_str() {
        let fetched: BTreeMap<String, String> = serde_json::from_str(text)?;
        state.asset_data.extend(fetched);
    }
    Ok(())
}
