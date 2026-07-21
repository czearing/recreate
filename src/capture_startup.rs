use crate::{
    capture::read_state,
    cdp::Cdp,
    model::{PageState, Viewport},
};
use anyhow::Result;
use std::time::Duration;

pub async fn wait_ready(cdp: &mut Cdp, wait_for_startup: bool) -> Result<()> {
    wait_ready_mode(cdp, wait_for_startup, true).await
}

pub async fn wait_ready_without_lifecycle(cdp: &mut Cdp, wait_for_startup: bool) -> Result<()> {
    wait_ready_mode(cdp, wait_for_startup, false).await
}

async fn wait_ready_mode(
    cdp: &mut Cdp,
    wait_for_startup: bool,
    wait_for_lifecycle: bool,
) -> Result<()> {
    let started = std::time::Instant::now();
    let mut previous = String::new();
    let mut stable = 0;
    for _ in 0..120 {
        let lifecycle = if wait_for_lifecycle {
            "window.__recreateLifecycleDone === true &&"
        } else {
            ""
        };
        let source = format!(
            r#"(() => {{
              const visible = Array.from(document.querySelectorAll('*'))
                .filter(element => {{
                  const rect = element.getBoundingClientRect();
                  const style = getComputedStyle(element);
                  return rect.width > 0 && rect.height > 0 &&
                    style.display !== 'none' && style.visibility !== 'hidden' &&
                    Number(style.opacity || 1) > 0;
                }})
                .slice(0, 80)
                .map(element => {{
                  const rect = element.getBoundingClientRect();
                  const style = getComputedStyle(element);
                  return [
                    element.tagName, Math.round(rect.x), Math.round(rect.y),
                    Math.round(rect.width), Math.round(rect.height),
                    style.display
                  ].join(':');
                }}).join('|');
              return {{
              ready: document.readyState === 'complete' &&
                document.fonts.status === 'loaded' &&
                {lifecycle}
                (window.__recreatePendingRequests || 0) === 0,
              signature: visible,
              blocking: Array.from(document.querySelectorAll('*')).some(element => {{
                const rect = element.getBoundingClientRect();
                const style = getComputedStyle(element);
                const area = rect.width * rect.height;
                const z = Number(style.zIndex);
                return area >= innerWidth * innerHeight * 0.9 &&
                  ['absolute','fixed'].includes(style.position) &&
                  Number.isFinite(z) && z >= 50 &&
                  style.pointerEvents !== 'none' &&
                  style.display !== 'none' && style.visibility !== 'hidden';
              }})
            }};
            }})()"#
        );
        let value = cdp.evaluate(&source).await?;
        let signature = value["signature"].as_str().unwrap_or_default();
        let startup_complete = !wait_for_startup || value["blocking"].as_bool() != Some(true);
        let ready = value["ready"].as_bool() == Some(true) && !signature.is_empty();
        if ready && startup_complete && started.elapsed() >= Duration::from_secs(5) {
            return Ok(());
        }
        if ready && startup_complete && signature == previous {
            stable += 1;
            if stable >= 3 {
                return Ok(());
            }
        } else {
            stable = 0;
        }
        previous = signature.to_string();
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    anyhow::bail!("page did not become stable")
}

pub fn ensure_settled(state: &PageState) -> Result<()> {
    let area = f64::from(state.viewport.width) * f64::from(state.viewport.height);
    if state.nodes.iter().any(|node| {
        let z = node
            .style
            .get("z-index")
            .and_then(|value| value.parse::<i32>().ok());
        node.rect.width * node.rect.height >= area * 0.9
            && matches!(
                node.style.get("position").map(String::as_str),
                Some("absolute" | "fixed")
            )
            && z.is_some_and(|value| value >= 50)
            && node.style.get("pointer-events").map(String::as_str) != Some("none")
            && node.style.get("display").map(String::as_str) != Some("none")
            && node.style.get("visibility").map(String::as_str) != Some("hidden")
    }) {
        anyhow::bail!("settled capture still contains a blocking overlay");
    }
    Ok(())
}

pub async fn wait_startup(
    cdp: &mut Cdp,
    viewport: &Viewport,
    started: std::time::Instant,
) -> Result<Option<(PageState, u64)>> {
    for _ in 0..60 {
        if cdp
            .evaluate(
                "(async()=>{const blocking=element=>{const rect=element.getBoundingClientRect(),\
                 style=getComputedStyle(element),z=Number(style.zIndex);return \
                 rect.width*rect.height>=innerWidth*innerHeight*.9&&\
                 ['absolute','fixed'].includes(style.position)&&Number.isFinite(z)&&z>=50&&\
                 style.pointerEvents!=='none'&&style.display!=='none'&&style.visibility!=='hidden'};\
                 const overlay=Array.from(document.querySelectorAll('*')).find(blocking);\
                 if(!overlay)return false;const images=[...(overlay.matches('img')?[overlay]:[]),\
                 ...overlay.querySelectorAll('img')];await Promise.race([Promise.all(\
                 images.map(image=>image.complete?\
                 (image.decode?image.decode().catch(()=>{}):Promise.resolve()):new Promise(resolve=>{\
                 image.addEventListener('load',resolve,{once:true});\
                 image.addEventListener('error',resolve,{once:true})}))),\
                 new Promise(resolve=>setTimeout(resolve,2000))]);return blocking(overlay)})()",
            )
            .await?
            .as_bool()
            == Some(true)
        {
            let state = read_state(cdp, viewport.clone()).await?;
            return Ok(Some((state, started.elapsed().as_millis() as u64)));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(None)
}

pub fn startup_nodes(state: &PageState, animation_targets: &[String]) -> Vec<crate::model::Node> {
    let area = f64::from(state.viewport.width) * f64::from(state.viewport.height);
    let mut roots: Vec<_> = state
        .nodes
        .iter()
        .filter(|node| {
            let z = node
                .style
                .get("z-index")
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or_default();
            node.rect.width * node.rect.height >= area * 0.9
                && matches!(
                    node.style.get("position").map(String::as_str),
                    Some("absolute" | "fixed")
                )
                && z >= 50
        })
        .map(|node| node.path.clone())
        .collect();
    for target in animation_targets {
        if state.nodes.iter().any(|node| node.path == *target) && !roots.contains(target) {
            roots.push(target.clone());
        }
    }
    let selected: std::collections::BTreeSet<_> = state
        .nodes
        .iter()
        .filter(|node| {
            roots
                .iter()
                .any(|root| node.path == *root || node.path.starts_with(&format!("{root}>")))
        })
        .map(|node| node.path.clone())
        .collect();
    state
        .nodes
        .iter()
        .filter(|node| selected.contains(&node.path))
        .cloned()
        .map(|mut node| {
            let original = node.path.clone();
            node.path = format!("startup>{original}");
            node.parent = node
                .parent
                .filter(|parent| selected.contains(parent))
                .map(|parent| format!("startup>{parent}"));
            node
        })
        .collect()
}
