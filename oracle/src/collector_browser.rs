use crate::{browser::Browser, model::Viewport};
use serde_json::json;
use tokio::time::{Duration, sleep};

const RENDERED_STATE: &str = r#"(() => {
  const meaningful = [...document.querySelectorAll('body *')].some(element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    if (rect.width <= 0 || rect.height <= 0 || style.display === 'none' ||
        style.visibility === 'hidden' || Number(style.opacity || 1) === 0) return false;
    if (element.textContent?.trim() ||
        ['IMG','SVG','CANVAS','VIDEO','IFRAME','INPUT','BUTTON'].includes(element.tagName)) return true;
    return style.backgroundImage !== 'none' ||
      !['rgba(0, 0, 0, 0)','transparent'].includes(style.backgroundColor);
  });
  return {body:!!document.body,ready:document.readyState,fonts:document.fonts.status,meaningful};
})()"#;

pub(crate) async fn wait_rendered(browser: &mut Browser) -> anyhow::Result<()> {
    for _ in 0..2400 {
        let state = browser.cdp.evaluate(RENDERED_STATE).await?;
        if state["body"] == true
            && state["ready"] == "complete"
            && state["fonts"] == "loaded"
            && state["meaningful"] == true
        {
            sleep(Duration::from_millis(300)).await;
            settle(browser).await?;
            return Ok(());
        }
        sleep(Duration::from_millis(25)).await;
    }
    anyhow::bail!("page did not produce observable rendered content")
}

pub(crate) async fn reload(browser: &mut Browser) -> anyhow::Result<()> {
    let previous_loader = loader_id(browser).await?;
    browser
        .cdp
        .send("Page.reload", serde_json::json!({}))
        .await?;
    for _ in 0..2400 {
        if let Ok(state) = browser.cdp.evaluate(RENDERED_STATE).await
            && loader_id(browser).await.ok().as_deref() != Some(previous_loader.as_str())
            && state["body"] == true
            && state["ready"] == "complete"
            && state["fonts"] == "loaded"
        {
            settle(browser).await?;
            return Ok(());
        }
        sleep(Duration::from_millis(25)).await;
    }
    anyhow::bail!("page reload did not replace the rendered document")
}

async fn loader_id(browser: &mut Browser) -> anyhow::Result<String> {
    browser
        .cdp
        .send("Page.getFrameTree", json!({}))
        .await?
        .pointer("/frameTree/frame/loaderId")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("page loader identity is unavailable"))
}

async fn settle(browser: &mut Browser) -> anyhow::Result<()> {
    let stable = browser
        .cdp
        .evaluate(
            r#"new Promise(resolve=>{
              let clean=0;
              const started=performance.now();
              const frame=()=>{
                const state=globalThis.__recreateOracle?.pending;
                const pending=(state?.fetches||0)>0||(state?.xhrs||0)>0;
                clean=pending?0:clean+1;
                if(clean>=2)resolve(true)
                else if(performance.now()-started>=2000)resolve(true)
                else requestAnimationFrame(frame);
              };
              requestAnimationFrame(frame);
            })"#,
        )
        .await?;
    anyhow::ensure!(
        stable == true,
        "rendered page did not reach a DOM fixed point"
    );
    Ok(())
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
        .evaluate("new Promise(requestAnimationFrame)")
        .await?;
    Ok(Viewport { width, height })
}
