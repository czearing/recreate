use crate::{browser::Browser, model::Viewport};
use serde_json::json;
use tokio::time::{Duration, sleep};

pub(crate) async fn reload(browser: &mut Browser) -> anyhow::Result<()> {
    let previous_origin = browser
        .cdp
        .evaluate("performance.timeOrigin")
        .await?
        .as_f64()
        .unwrap_or_default();
    browser
        .cdp
        .send("Page.reload", serde_json::json!({}))
        .await?;
    for _ in 0..200 {
        if let Ok(state) = browser
            .cdp
            .evaluate(
                "({body:!!document.body,ready:document.readyState,\
                 origin:performance.timeOrigin})",
            )
            .await
        {
            let replaced = state["origin"]
                .as_f64()
                .is_some_and(|origin| (origin - previous_origin).abs() > 0.5);
            if replaced && state["body"] == true && state["ready"] == "complete" {
                browser
                    .cdp
                    .evaluate(
                        "new Promise(resolve => requestAnimationFrame(() => \
                         requestAnimationFrame(resolve)))",
                    )
                    .await?;
                return Ok(());
            }
        }
        sleep(Duration::from_millis(10)).await;
    }
    anyhow::bail!("page reload did not replace the document")
}

pub(crate) async fn advance(browser: &mut Browser, milliseconds: u32) -> anyhow::Result<()> {
    browser
        .cdp
        .send(
            "Emulation.setVirtualTimePolicy",
            json!({
                "policy":"pauseIfNetworkFetchesPending",
                "budget":milliseconds,
                "maxVirtualTimeTaskStarvationCount":10_000
            }),
        )
        .await?;
    browser
        .cdp
        .wait_event("Emulation.virtualTimeBudgetExpired")
        .await?;
    Ok(())
}

pub(crate) async fn resize(
    browser: &mut Browser,
    width: u32,
    height: u32,
) -> anyhow::Result<Viewport> {
    browser
        .cdp
        .send(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":width,"height":height,"deviceScaleFactor":1,"mobile":false}),
        )
        .await?;
    browser
        .cdp
        .evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
        .await?;
    Ok(Viewport { width, height })
}
