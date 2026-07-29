use super::{
    interactions_focus::{begin_scope, take_scope},
    interactions_graph::{edge_candidate_action, visual_state_matches},
    interactions_runtime::{restoration_requires_reload, restore, settle},
    interactions_scripts::{CANDIDATES, Candidate, PREFLIGHT},
};
use crate::{
    capture,
    cdp::Cdp,
    interaction_state,
    interactions_input::{hover_matching, leave_pointer},
    model::{Interaction, InteractionAction, InteractionTransition, PageState},
};
use anyhow::Result;

pub(super) async fn capture_hover_transitions(
    cdp: &mut Cdp,
    baseline: &PageState,
    interactions: &mut Vec<Interaction>,
    transitions: &mut Vec<InteractionTransition>,
) -> Result<()> {
    let fresh = restore(cdp, baseline, false).await?;
    let candidates: Vec<Candidate> = serde_json::from_value(cdp.evaluate(CANDIDATES).await?)?;
    for (index, target) in candidates
        .into_iter()
        .filter(|candidate| !candidate.disabled)
        .enumerate()
    {
        if index > 0 && index % 10 == 0 {
            eprintln!("captured hover probes={index}");
        }
        cdp.take_events();
        let before = cdp.evaluate(PREFLIGHT).await?;
        begin_scope(cdp, &target.path).await?;
        if !hover_matching(cdp, &target.path).await? {
            let _ = take_scope(cdp).await?;
            continue;
        }
        let settled = settle(cdp, false).await?;
        let affected = take_scope(cdp).await?;
        if cdp.evaluate(PREFLIGHT).await? == before {
            continue;
        }
        let mut hovered =
            capture::read_visual_interaction_state(cdp, baseline.viewport.clone()).await?;
        crate::interaction_rebase::causal(&mut hovered, &fresh, &affected);
        crate::interaction_rebase::unchanged(&mut hovered, &fresh, baseline);
        interaction_state::compact(&mut hovered, &fresh, settled);
        let destination = interactions
            .iter()
            .enumerate()
            .find(|(_, interaction)| {
                interaction
                    .states
                    .iter()
                    .find(|state| state.viewport.width == baseline.viewport.width)
                    .is_some_and(|state| visual_state_matches(&hovered, state))
            })
            .map(|(index, _)| index + 1)
            .unwrap_or_else(|| {
                interactions.push(Interaction {
                    trigger_path: target.path.clone(),
                    trigger_tag: target.tag.clone(),
                    trigger_label: target.label.clone(),
                    trigger_occurrence: Some(target.occurrence),
                    focused_path: None,
                    states: vec![hovered],
                });
                interactions.len()
            });
        transitions.push(edge_candidate_action(
            0,
            destination,
            &target,
            InteractionAction::Hover,
        ));
        if let Some(activated) = transitions
            .iter()
            .find(|transition| {
                transition.from_state == 0
                    && transition.action == InteractionAction::Activate
                    && transition.trigger_path == target.path
                    && transition.trigger_tag == target.tag
                    && transition.trigger_label == target.label
                    && transition.trigger_occurrence == Some(target.occurrence)
            })
            .map(|transition| transition.to_state)
        {
            transitions.push(edge_candidate_action(
                destination,
                activated,
                &target,
                InteractionAction::Activate,
            ));
        }
        begin_scope(cdp, &target.path).await?;
        leave_pointer(cdp).await?;
        let settled = settle(cdp, false).await?;
        let affected = take_scope(cdp).await?;
        let mut left =
            capture::read_visual_interaction_state(cdp, baseline.viewport.clone()).await?;
        crate::interaction_rebase::causal(&mut left, &fresh, &affected);
        crate::interaction_rebase::unchanged(&mut left, &fresh, baseline);
        interaction_state::compact(&mut left, &fresh, settled);
        if !restoration_requires_reload(&left, baseline) {
            transitions.push(edge_candidate_action(
                destination,
                0,
                &target,
                InteractionAction::Leave,
            ));
        } else {
            restore(cdp, baseline, false).await?;
        }
    }
    Ok(())
}
