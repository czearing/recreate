use crate::{browser::Browser, probe_discovery};
use serde_json::json;

pub async fn activate(browser: &mut Browser, anchor: &str) -> anyhow::Result<()> {
    let (x, y) = point(browser, anchor).await?;
    begin(browser).await?;
    hover_point(browser, x, y).await?;
    mark_point(browser, x, y).await?;
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
    begin(browser).await?;
    hover_point(browser, x, y).await?;
    mark_point(browser, x, y).await?;
    settle(browser).await
}

pub async fn key(browser: &mut Browser, key: &str) -> anyhow::Result<()> {
    begin(browser).await?;
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

async fn mark_point(browser: &mut Browser, x: f64, y: f64) -> anyhow::Result<()> {
    browser
        .cdp
        .evaluate(&format!(
            "globalThis.__recreateOracleAction?.mark(document.elementFromPoint({x},{y}),true)"
        ))
        .await?;
    Ok(())
}

async fn settle(browser: &mut Browser) -> anyhow::Result<()> {
    browser
        .cdp
        .evaluate(
            r#"new Promise(resolve => {
              const action=globalThis.__recreateOracleAction;
              let clean=0;
              const finish=()=>{
                action?.observer.disconnect();
                action?.layoutObserver?.disconnect();
                removeEventListener('scroll',action?.scroll,true);
                removeEventListener('focusin',action?.focus,true);
                const elapsed=performance.now()-(action?.started||performance.now());
                globalThis.__recreateOracleActionResult={
                  stable:elapsed<=450?'fast':'slow'
                };
                resolve();
              };
              const frame=()=>{
                const state=globalThis.__recreateOracle?.pending;
                const pendingTimeout=[...(state?.timeouts?.keys?.()||[])]
                  .some(id=>!action?.timeouts.has(id));
                const pending=(state?.fetches||0)>0||(state?.xhrs||0)>0||pendingTimeout;
                const running=document.getAnimations().some(item=>
                  !action?.animations.has(item)&&item.playState==='running');
                clean=pending||running||action?.dirty?0:clean+1;
                if(action)action.dirty=false;
                if(clean>=1||performance.now()-(action?.started||0)>=500)finish();
                else requestAnimationFrame(frame);
              };
              requestAnimationFrame(frame);
            })"#,
        )
        .await?;
    Ok(())
}

async fn begin(browser: &mut Browser) -> anyhow::Result<()> {
    browser
        .cdp
        .evaluate(
            r#"(() => {
              globalThis.__recreateOracleAction?.observer?.disconnect();
              const action={started:performance.now(),first:null,dirty:false,observer:null,
                layoutObserver:null,scroll:null,focus:null,dirtyNodes:new Set(),dirtyTrees:new Set(),
                animations:new Set(document.getAnimations()),
                timeouts:new Set(globalThis.__recreateOracle?.pending?.timeouts?.keys?.()||[])};
              action.mark=(node,tree=false)=>{
                if(node?.nodeType===Node.TEXT_NODE)node=node.parentElement;
                if(node?.nodeType===Node.ELEMENT_NODE)
                  (tree?action.dirtyTrees:action.dirtyNodes).add(node);
              };
              action.observer=new MutationObserver(records=>{
                action.dirty=true;
                action.first ??= performance.now();
                for(const record of records){
                  action.mark(record.target);
                  for(const node of record.addedNodes)action.mark(node,true);
                }
              });
              action.observer.observe(document,{attributes:true,childList:true,
                characterData:true,subtree:true});
              action.scroll=event=>action.mark(
                event.target===document?document.scrollingElement:event.target);
              action.focus=event=>action.mark(event.target);
              addEventListener('scroll',action.scroll,true);
              addEventListener('focusin',action.focus,true);
              if(globalThis.PerformanceObserver){
                try{
                  action.layoutObserver=new PerformanceObserver(list=>{
                    for(const entry of list.getEntries())
                      for(const source of entry.sources||[])action.mark(source.node);
                  });
                  action.layoutObserver.observe({type:'layout-shift',buffered:false});
                }catch{}
              }
              globalThis.__recreateOracleAction=action;
              globalThis.__recreateOracleActionResult=null;
            })()"#,
        )
        .await?;
    Ok(())
}
