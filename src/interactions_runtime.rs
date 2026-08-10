use super::{
    interactions_activate::activate,
    interactions_evidence::geometry_differs,
    interactions_scripts::{Candidate, SETTLE},
};
use crate::{browser, capture, cdp::Cdp, interaction_state, model::PageState};
use anyhow::Result;

pub(super) async fn close(cdp: &mut Cdp, candidate: &Candidate) -> Result<()> {
    if candidate.uses_text_entry() {
        return Ok(());
    }
    for event_type in ["keyDown", "keyUp"] {
        cdp.send(
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type":event_type,
                "key":"Escape",
                "code":"Escape",
                "windowsVirtualKeyCode":27
            }),
        )
        .await?;
    }
    let _ = settle(cdp, false).await?;
    let expanded = cdp
        .evaluate(&format!(
            "document.querySelector({})?.getAttribute('aria-expanded')==='true'",
            serde_json::to_string(&candidate.path)?
        ))
        .await?
        == true;
    if !expanded {
        return Ok(());
    }
    if activate(cdp, candidate).await? {
        let _ = settle(cdp, false).await?;
    }
    Ok(())
}

pub(super) async fn settle(cdp: &mut Cdp, text_entry: bool) -> Result<bool> {
    let timeout = if text_entry { 1_500 } else { 500 };
    let source = SETTLE.replace(">= 500", &format!(">= {timeout}"));
    Ok(cdp.evaluate(&source).await?.as_bool() == Some(true))
}

pub(super) async fn restore(
    cdp: &mut Cdp,
    baseline: &PageState,
    reload: bool,
) -> Result<PageState> {
    let same_url = cdp.evaluate("location.href").await?.as_str() == Some(baseline.url.as_str());
    if same_url && !reload {
        let controls = baseline
            .nodes
            .iter()
            .filter(|node| {
                node.attributes.contains_key("aria-selected")
                    || node.attributes.contains_key("aria-pressed")
            })
            .map(|node| {
                serde_json::json!({
                    "path":node.path,
                    "selected":node.attributes.get("aria-selected"),
                    "pressed":node.attributes.get("aria-pressed")
                })
            })
            .collect::<Vec<_>>();
        cdp.evaluate(&format!(
            "(()=>{{const controls={};\
             document.dispatchEvent(new KeyboardEvent('keydown',{{key:'Escape',bubbles:true}}));\
             for(const element of document.querySelectorAll('[aria-expanded=\"true\"]')){{\
               element.click()\
             }}\
             for(const state of controls){{\
               const element=document.querySelector(state.path);if(!element)continue;\
               if(state.selected==='true'&&element.getAttribute('aria-selected')!=='true'){{\
                 element.click();break\
               }}\
             }}\
             for(const state of controls){{\
               const element=document.querySelector(state.path);if(!element||state.pressed==null)continue;\
               if(element.getAttribute('aria-pressed')!==state.pressed)element.click()\
             }}\
             for(const element of document.querySelectorAll('*')){{\
               if(element.scrollLeft||element.scrollTop)element.scrollTo(0,0)\
             }}scrollTo(0,0);document.activeElement?.blur()}})()",
            serde_json::to_string(&controls)?
        ))
        .await?;
        browser::set_viewport(cdp, baseline.viewport.width, baseline.viewport.height).await?;
        wait_frames(cdp).await?;
        let restored = capture::read_interaction_state(cdp, baseline.viewport.clone()).await?;
        if !restoration_requires_reload(&restored, baseline) {
            return Ok(restored);
        }
    }
    if same_url {
        capture::prepare_interaction_state(cdp, &baseline.viewport, true).await?;
    } else {
        cdp.send("Page.navigate", serde_json::json!({"url":baseline.url}))
            .await?;
        let _ = capture::capture_state(cdp, baseline.viewport.clone(), false).await?;
    }
    cdp.evaluate("scrollTo(0,0)").await?;
    wait_frames(cdp).await?;
    capture::read_interaction_state(cdp, baseline.viewport.clone()).await
}

pub(super) async fn wait_frames(cdp: &mut Cdp) -> Result<()> {
    cdp.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))")
        .await?;
    Ok(())
}

pub(super) fn restoration_requires_reload(restored: &PageState, baseline: &PageState) -> bool {
    interaction_state::selected_differs(restored, baseline)
        || interaction_state::surface_differs(restored, baseline, "", "")
        || geometry_differs(restored, baseline)
}
