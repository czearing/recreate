use crate::model::{BrowserCookie, PageState, Viewport};
use anyhow::{Context, Result};
use serde_json::json;

pub async fn read_state(cdp: &mut crate::cdp::Cdp, viewport: Viewport) -> Result<PageState> {
    let sheets = crate::capture::authored_sheets::collect(cdp).await;
    read(
        cdp,
        viewport,
        &crate::page_script::source_with_sheets(&sheets.sheets),
        "capture script returned non-string",
    )
    .await
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;

pub async fn read_interaction_state(
    cdp: &mut crate::cdp::Cdp,
    viewport: Viewport,
) -> Result<PageState> {
    read(
        cdp,
        viewport,
        &crate::interaction_script::source(),
        "interaction capture returned non-string",
    )
    .await
}

async fn read(
    cdp: &mut crate::cdp::Cdp,
    viewport: Viewport,
    source: &str,
    error: &str,
) -> Result<PageState> {
    let raw = cdp.evaluate(source).await?;
    let text = raw.as_str().with_context(|| error.to_string())?;
    let mut state: PageState = serde_json::from_str(text)?;
    state.viewport = viewport;
    Ok(state)
}

pub(in crate::capture) async fn browser_cookies(cdp: &mut crate::cdp::Cdp) -> Vec<BrowserCookie> {
    cdp.send("Network.getAllCookies", json!({}))
        .await
        .ok()
        .and_then(|value| serde_json::from_value(value["cookies"].clone()).ok())
        .unwrap_or_default()
}
