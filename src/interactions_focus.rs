use super::{
    interactions_graph::{edge_candidate_action, visual_state_matches},
    interactions_runtime::{activate, restore, settle},
    interactions_scripts::{ACTION_SCOPE, CANDIDATES, Candidate, PREFLIGHT, TAKE_SCOPE},
};
use crate::{
    capture,
    cdp::Cdp,
    interaction_state,
    interactions_input::focused_path,
    model::{Interaction, InteractionAction, InteractionTransition, PageState},
};
use anyhow::Result;

const MAX_FOCUS_PROBES: usize = 20;

pub(super) async fn capture_focus_transitions(
    cdp: &mut Cdp,
    baseline: &PageState,
    interactions: &mut Vec<Interaction>,
    transitions: &mut Vec<InteractionTransition>,
) -> Result<()> {
    let fresh = restore(cdp, baseline, false).await?;
    cdp.evaluate(
        "document.body.tabIndex=-1;document.body.focus({preventScroll:true});\
         document.body.removeAttribute('tabindex')",
    )
    .await?;
    let candidates: Vec<Candidate> = serde_json::from_value(cdp.evaluate(CANDIDATES).await?)?;
    let mut from_state = 0;
    let mut visited = std::collections::HashSet::new();
    let mut focused_states = Vec::new();
    for _ in 0..candidates.len().saturating_add(4).min(MAX_FOCUS_PROBES) {
        cdp.take_events();
        let before = cdp.evaluate(PREFLIGHT).await?;
        begin_scope(cdp, "").await?;
        press_key(cdp, "Tab", "Tab", 9).await?;
        let settled = settle(cdp, false).await?;
        let affected = take_scope(cdp).await?;
        let Some(path) = focused_path(cdp).await? else {
            continue;
        };
        if !visited.insert(path.clone()) {
            break;
        }
        let Some(target) = candidates
            .iter()
            .find(|candidate| candidate.path == path && !candidate.disabled)
        else {
            continue;
        };
        if cdp.evaluate(PREFLIGHT).await? == before {
            continue;
        }
        let mut focused =
            capture::read_visual_interaction_state(cdp, baseline.viewport.clone()).await?;
        crate::interaction_rebase::causal(&mut focused, &fresh, &affected);
        crate::interaction_rebase::unchanged(&mut focused, &fresh, baseline);
        interaction_state::compact(&mut focused, &fresh, settled);
        let destination = interactions
            .iter()
            .enumerate()
            .find(|(_, interaction)| {
                interaction
                    .states
                    .iter()
                    .find(|state| state.viewport.width == baseline.viewport.width)
                    .is_some_and(|state| visual_state_matches(&focused, state))
            })
            .map(|(index, _)| index + 1)
            .unwrap_or_else(|| {
                interactions.push(Interaction {
                    trigger_path: target.path.clone(),
                    trigger_tag: target.tag.clone(),
                    trigger_label: target.label.clone(),
                    trigger_occurrence: Some(target.occurrence),
                    focused_path: Some(target.path.clone()),
                    states: vec![focused],
                });
                interactions.len()
            });
        transitions.push(edge_candidate_action(
            from_state,
            destination,
            target,
            InteractionAction::Focus,
        ));
        focused_states.push((destination, target.clone()));
        from_state = destination;
    }
    for (focus_state, target) in focused_states {
        let destination = transitions
            .iter()
            .find(|transition| {
                transition.from_state == 0
                    && transition.action == InteractionAction::Activate
                    && transition.trigger_path == target.path
                    && transition.trigger_tag == target.tag
                    && transition.trigger_label == target.label
                    && transition.trigger_occurrence == Some(target.occurrence)
            })
            .map(|transition| transition.to_state);
        if let Some(destination) = destination {
            transitions.push(edge_candidate_action(
                focus_state,
                destination,
                &target,
                InteractionAction::Activate,
            ));
        }
    }
    Ok(())
}

pub(super) async fn press_key(
    cdp: &mut Cdp,
    key: &str,
    code: &str,
    virtual_key: u32,
) -> Result<()> {
    for event_type in ["keyDown", "keyUp"] {
        cdp.send(
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type":event_type,
                "key":key,
                "code":code,
                "windowsVirtualKeyCode":virtual_key
            }),
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn begin_scope(cdp: &mut Cdp, trigger: &str) -> Result<()> {
    cdp.evaluate(&format!(
        "({ACTION_SCOPE})({})",
        serde_json::to_string(trigger)?
    ))
    .await?;
    Ok(())
}

pub(super) async fn take_scope(cdp: &mut Cdp) -> Result<Vec<String>> {
    Ok(serde_json::from_value(cdp.evaluate(TAKE_SCOPE).await?)?)
}

pub(super) async fn reach(
    cdp: &mut Cdp,
    baseline: &PageState,
    prefix: &[Candidate],
) -> Result<Option<(PageState, PageState)>> {
    let fresh = restore(cdp, baseline, false).await?;
    for action in prefix {
        if !activate(cdp, action).await? {
            return Ok(None);
        }
        let _ = settle(cdp, action.uses_text_entry()).await?;
    }
    let reached = capture::read_interaction_state(cdp, baseline.viewport.clone()).await?;
    Ok(Some((fresh, reached)))
}
