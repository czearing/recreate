use crate::{browser::Browser, probe_discovery};
use serde_json::json;

pub async fn activate(browser: &mut Browser, anchor: &str) -> anyhow::Result<()> {
    let (x, y) = point(browser, anchor).await?;
    hover_point(browser, x, y).await?;
    browser
        .cdp
        .send(
            "Input.dispatchMouseEvent",
            json!({"type": "mousePressed", "x": x, "y": y, "button": "left", "clickCount": 1}),
        )
        .await?;
    browser
        .cdp
        .send(
            "Input.dispatchMouseEvent",
            json!({"type": "mouseReleased", "x": x, "y": y, "button": "left", "clickCount": 1}),
        )
        .await?;
    settle(browser).await
}

pub async fn hover(browser: &mut Browser, anchor: &str) -> anyhow::Result<()> {
    let (x, y) = point(browser, anchor).await?;
    hover_point(browser, x, y).await?;
    settle(browser).await
}

pub async fn key(browser: &mut Browser, key: &str) -> anyhow::Result<()> {
    for kind in ["keyDown", "keyUp"] {
        browser
            .cdp
            .send(
                "Input.dispatchKeyEvent",
                json!({"type": kind, "key": key, "code": key}),
            )
            .await?;
    }
    settle(browser).await
}

async fn point(browser: &mut Browser, anchor: &str) -> anyhow::Result<(f64, f64)> {
    let expression = format!(
        "({})({})",
        probe_discovery::FIND_ANCHOR,
        serde_json::to_string(anchor)?
    );
    let point = browser.cdp.evaluate(&expression).await?;
    let x = point["x"]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("source anchor is absent in candidate: {anchor}"))?;
    let y = point["y"]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("source anchor has no hit point: {anchor}"))?;
    Ok((x, y))
}

async fn hover_point(browser: &mut Browser, x: f64, y: f64) -> anyhow::Result<()> {
    browser
        .cdp
        .send(
            "Input.dispatchMouseEvent",
            json!({"type": "mouseMoved", "x": x, "y": y}),
        )
        .await?;
    Ok(())
}

async fn settle(browser: &mut Browser) -> anyhow::Result<()> {
    browser
        .cdp
        .evaluate("new Promise(r => requestAnimationFrame(() => queueMicrotask(r)))")
        .await?;
    Ok(())
}
