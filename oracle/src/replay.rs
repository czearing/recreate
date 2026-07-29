use crate::{browser::Browser, replay_action, transition_probe};
use serde_json::json;

pub async fn activate(browser: &mut Browser, anchor: &str) -> anyhow::Result<serde_json::Value> {
    let (x, y) = replay_action::begin_at(browser, anchor).await?;
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
    drain_network(browser).await?;
    settle(browser).await
}

pub async fn hover(browser: &mut Browser, anchor: &str) -> anyhow::Result<serde_json::Value> {
    let (x, y) = replay_action::begin_at(browser, anchor).await?;
    hover_point(browser, x, y).await?;
    drain_network(browser).await?;
    settle(browser).await
}

pub async fn key(browser: &mut Browser, key: &str) -> anyhow::Result<serde_json::Value> {
    replay_action::begin(browser).await?;
    for kind in ["keyDown", "keyUp"] {
        browser
            .cdp
            .send(
                "Input.dispatchKeyEvent",
                json!({"type": kind, "key": key, "code": key}),
            )
            .await?;
    }
    drain_network(browser).await?;
    settle(browser).await
}

pub async fn neutralize(browser: &mut Browser) -> anyhow::Result<()> {
    browser
        .cdp
        .send(
            "Input.dispatchMouseEvent",
            json!({"type":"mouseMoved","x":-1,"y":-1}),
        )
        .await?;
    browser
        .cdp
        .evaluate("document.activeElement?.blur();true")
        .await?;
    Ok(())
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

async fn drain_network(browser: &mut Browser) -> anyhow::Result<()> {
    if !browser.has_network_fixture() {
        return Ok(());
    }
    for _ in 0..3 {
        browser.cdp.evaluate("0").await?;
        browser.fulfill_network_fixture().await?;
        tokio::task::yield_now().await;
    }
    Ok(())
}

async fn settle(browser: &mut Browser) -> anyhow::Result<serde_json::Value> {
    let expression = r#"new Promise(resolve => {
              const action=globalThis.__recreateOracleAction;
              const finish=()=>{
                action?.observer.disconnect();
                action?.layoutObserver?.disconnect();
                removeEventListener('scroll',action?.scroll,true);
                removeEventListener('focusin',action?.focus,true);
                for(const type of ['mousemove','mousedown','keydown'])
                  removeEventListener(type,action?.inputEvent,true);
                removeEventListener('error',action?.error);
                removeEventListener('unhandledrejection',action?.rejection);
                if(action?.consoleError)console.error=action.consoleError;
                if(action?.fetch)globalThis.fetch=action.fetch;
                if(action?.xhrOpen)XMLHttpRequest.prototype.open=action.xhrOpen;
                const origin=action?.input||action?.started||performance.now();
                const elapsed=performance.now()-origin;
                const first=action?.first==null?null:Math.max(0,action.first-origin);
                const timing=value=>value==null?'none':value<=100?'immediate':
                  value<=250?'responsive':value<=450?'delayed':'slow';
                globalThis.__recreateOracleActionResult={
                  first:timing(first),settled:action?.waited?timing(elapsed):'immediate',
                  stable:elapsed<=450?'fast':'slow',errors:action?.errors||0,
                  unsafe:action?.unsafe||0
                };
                resolve(__CAPTURE__);
              };
              const check=()=>{
                const state=globalThis.__recreateOracle?.pending;
                const pendingTimeout=[...(state?.timeouts?.keys?.()||[])]
                  .some(id=>!action?.timeouts.has(id));
                const pending=(state?.fetches||0)>0||(state?.xhrs||0)>0||pendingTimeout;
                const running=document.getAnimations().some(item=>
                  !action?.animations.has(item)&&item.playState==='running');
                 if(running&&action)action.first??=performance.now();
                if((pending||running)&&action)action.waited=true;
                if((!pending&&!running)||performance.now()-(action?.started||0)>=500)finish();
                else requestAnimationFrame(check);
              };
              queueMicrotask(check);
            })"#
    .replace("__CAPTURE__", transition_probe::CAPTURE);
    browser.cdp.evaluate(&expression).await
}
