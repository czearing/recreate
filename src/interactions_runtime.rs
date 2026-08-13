use super::{
    interactions_activate::activate,
    interactions_evidence::geometry_differs,
    interactions_scripts::{self, Candidate, PREFLIGHT},
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
    let deadline = if text_entry { 1_500 } else { 500 };
    let source = interactions_scripts::settle(deadline);
    Ok(cdp.evaluate(&source).await?.as_bool() == Some(true))
}

/// A page driven back to its baseline is the same page every time, so the state read there
/// describes the baseline rather than the candidate that ran before it. Reading it once per
/// candidate recomputes an invariant, and the whole-document style measurement inside that read
/// is what makes the sweep scale with page size instead of with what the action changed.
/// `PREFLIGHT` is already trusted to decide whether an action moved the page; the same witness
/// decides whether the page came back to one already read, at one evaluation instead of a full
/// read. The witness is taken after the viewport is set, so it carries that viewport's geometry,
/// and entries are keyed by viewport as well so a narrow arm can never answer for a wide one.
/// Only a page at rest can be remembered. A running animation or transition changes what the
/// state read records while every attribute, rect and computed value the witness compares stays
/// equal, so a page still in motion would be remembered mid-flight and that one frame replayed
/// into every later state. Whether the page is moving is asked of the engine rather than guessed
/// from a property list, and costs one count regardless of page size.
#[derive(Default)]
pub(super) struct RestingStates {
    entries: Vec<((u32, u32), serde_json::Value, PageState)>,
}

/// `getAnimations` reports CSS animations, CSS transitions and script-driven animations alike, so
/// one question covers every way a page can still be changing without an attribute changing.
const AT_REST: &str = "document.getAnimations().length===0";

impl RestingStates {
    pub(super) fn recall(
        &self,
        size: (u32, u32),
        witness: &serde_json::Value,
    ) -> Option<&PageState> {
        self.entries
            .iter()
            .find(|(key, seen, _)| *key == size && seen == witness)
            .map(|(_, _, state)| state)
    }

    pub(super) fn record(
        &mut self,
        size: (u32, u32),
        witness: serde_json::Value,
        state: &PageState,
    ) {
        self.entries.retain(|(key, _, _)| *key != size);
        self.entries.push((size, witness, state.clone()));
    }
}

pub(super) async fn restore(
    cdp: &mut Cdp,
    rest: &mut RestingStates,
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
        let size = (baseline.viewport.width, baseline.viewport.height);
        let at_rest = cdp.evaluate(AT_REST).await? == true;
        let witness = cdp.evaluate(PREFLIGHT).await?;
        if at_rest && let Some(state) = rest.recall(size, &witness) {
            return Ok(state.clone());
        }
        let restored = capture::read_interaction_state(cdp, baseline.viewport.clone()).await?;
        if !restoration_requires_reload(&restored, baseline) {
            if at_rest {
                rest.record(size, witness, &restored);
            }
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
