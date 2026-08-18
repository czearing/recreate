use crate::{
    capture,
    cdp::Cdp,
    interactions_input, lifecycle_script,
    model::{Interaction, PageState},
};
use anyhow::Result;
use serde_json::Value;

const ANIMATIONS: &str = r#"
(() => {
__NODE_PATH__
  return document.getAnimations({subtree:true}).map(animation => {
    const timing = animation.effect?.getTiming?.() || {};
    return {
      target: animation.effect?.target ? pathOf(animation.effect.target) : '',
      keyframes: animation.effect?.getKeyframes?.() || [],
      timing: {
        ...timing,
        iterations: timing.iterations === Infinity ? 'infinite' : timing.iterations,
        playState: animation.playState,
        playbackRate: animation.playbackRate
      }
    };
  }).filter(animation => animation.target);
})()
"#;

/// The animation reader, with the shared path definition spliced in.
pub fn animations_script() -> String {
    crate::node_path::embed(ANIMATIONS)
}

pub async fn state(
    cdp: &mut Cdp,
    expected: &PageState,
    trigger: Option<&Interaction>,
) -> Result<PageState> {
    cdp.enable(&["Page", "Runtime", "Network", "DOM", "CSS"])
        .await?;
    crate::capture::set_focus(cdp).await?;
    cdp.send(
        "Emulation.setEmulatedMedia",
        serde_json::json!({
            "features":[{
                "name":"prefers-reduced-motion",
                "value":"no-preference"
            }]
        }),
    )
    .await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        serde_json::json!({ "source": lifecycle_script::source() }),
    )
    .await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        serde_json::json!({ "source": "window.__recreateFreezeSequences=true" }),
    )
    .await?;
    cdp.send("Page.reload", serde_json::json!({"ignoreCache":false}))
        .await?;
    let Some(trigger) = trigger else {
        return capture::capture_state(cdp, expected.viewport.clone(), true).await;
    };
    capture::prepare_interaction_state(cdp, &expected.viewport, true).await?;
    cdp.evaluate("scrollTo(0,0)").await?;
    let activated = if interactions_input::text_entry(&trigger.trigger_tag) {
        interactions_input::submit_text_matching(
            cdp,
            &trigger.trigger_path,
            &trigger.trigger_tag,
            &trigger.trigger_label,
            trigger.trigger_occurrence,
        )
        .await?
    } else {
        interactions_input::click_matching(
            cdp,
            &trigger.trigger_path,
            &trigger.trigger_tag,
            &trigger.trigger_label,
            trigger.trigger_occurrence,
            true,
        )
        .await?
    };
    if !activated {
        let controls = cdp
            .evaluate(
                "Array.from(document.querySelectorAll('[data-recreate-control]')).map(\
                 element=>({tag:element.tagName.toLowerCase(),\
                 label:element.getAttribute('aria-label')||element.innerText||''}))",
            )
            .await?;
        anyhow::bail!(
            "interaction trigger was not found: {} controls={controls}",
            trigger.trigger_label
        );
    }
    cdp.send(
        "Input.dispatchMouseEvent",
        serde_json::json!({"type":"mouseMoved","x":-100,"y":-100}),
    )
    .await?;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let mut state = capture::read_interaction_state(cdp, expected.viewport.clone()).await?;
    let animations: Value = cdp.evaluate(&animations_script()).await?;
    state.animations = serde_json::from_value(animations)?;
    Ok(state)
}
