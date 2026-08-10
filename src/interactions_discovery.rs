use super::{
    interactions_activate::activate,
    interactions_evidence::{candidate_relevant, overlay_state},
    interactions_graph::{candidate, edge, edge_candidate, same_edge, state_matches},
    interactions_runtime::{restoration_requires_reload, settle},
    interactions_scope::{begin_scope, reach, take_scope},
    interactions_scripts::{CANDIDATES, Candidate},
};
use crate::{
    capture,
    cdp::Cdp,
    interaction_state,
    interactions_input::focused_path,
    model::{Interaction, InteractionTransition, PageState},
};
use anyhow::Result;

pub(super) async fn discover_transitions(
    cdp: &mut Cdp,
    baselines: &[PageState],
    interactions: &mut Vec<Interaction>,
) -> Result<Vec<InteractionTransition>> {
    let mut transitions = Vec::new();
    let activation_count = interactions.len();
    for (target_index, target) in interactions.iter().enumerate() {
        transitions.push(edge(0, target_index + 1, target));
    }
    let Some(baseline) = baselines.first() else {
        return Ok(transitions);
    };
    if std::env::var_os("RECREATE_SHALLOW_INTERACTIONS").is_some() {
        return Ok(transitions);
    }
    let mut prefixes = interactions[..activation_count]
        .iter()
        .map(|interaction| vec![candidate(interaction, baseline)])
        .collect::<Vec<_>>();
    let mut state_index = 0;
    while state_index < prefixes.len() && state_index < 12 && transitions.len() < 128 {
        let prefix = prefixes[state_index].clone();
        let Some((entry, reached)) = reach(cdp, baseline, &prefix).await? else {
            state_index += 1;
            continue;
        };
        let candidates: Vec<Candidate> = serde_json::from_value(cdp.evaluate(CANDIDATES).await?)?;
        let trigger = prefix.last().map(|candidate| candidate.path.as_str());
        let popup = prefix
            .last()
            .is_some_and(|candidate| !candidate.state_control)
            && overlay_state(&reached, baseline);
        let mut candidates = candidates
            .into_iter()
            .filter(|candidate| {
                !candidate.disabled
                    && !candidate.navigates
                    && candidate_relevant(candidate, baseline, &reached, trigger, popup)
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| {
            if popup {
                (trigger != Some(candidate.path.as_str()), false)
            } else {
                (
                    !candidate.state_control,
                    trigger != Some(candidate.path.as_str()),
                )
            }
        });
        let discovered = candidates.len();
        candidates.truncate(if popup { 2 } else { 8 });
        eprintln!(
            "exploring interaction state {} with {}/{} affected controls",
            state_index + 1,
            candidates.len(),
            discovered
        );
        // The page is already standing at the end of this prefix, so the first target is probed
        // from where enumeration left it. Only a later target needs the page driven back.
        let mut standing = Some(entry);
        for target in candidates {
            cdp.take_events();
            let fresh = match standing.take() {
                Some(state) => state,
                None => match reach(cdp, baseline, &prefix).await? {
                    Some((fresh, _)) => fresh,
                    None => continue,
                },
            };
            begin_scope(cdp, &target.path).await?;
            if !activate(cdp, &target).await? {
                let _ = take_scope(cdp).await?;
                continue;
            }

            let settled = settle(cdp, target.uses_text_entry()).await?;
            let affected = take_scope(cdp).await?;
            let mut result =
                capture::read_interaction_state(cdp, baseline.viewport.clone()).await?;
            crate::interaction_rebase::causal(&mut result, &fresh, &affected);
            crate::interaction_rebase::unchanged(&mut result, &fresh, baseline);
            interaction_state::compact(&mut result, &fresh, settled);
            let destination = if !restoration_requires_reload(&result, baseline) {
                Some(0)
            } else {
                interactions
                    .iter()
                    .enumerate()
                    .find(|(_, interaction)| {
                        interaction
                            .states
                            .iter()
                            .find(|state| state.viewport.width == baseline.viewport.width)
                            .is_some_and(|state| state_matches(&result, state))
                    })
                    .map(|(index, _)| index + 1)
            };
            let destination = if let Some(destination) = destination {
                destination
            } else if interactions.len() < 32 {
                let focused_path = focused_path(cdp).await?;
                interactions.push(Interaction {
                    trigger_path: target.path.clone(),
                    trigger_tag: target.tag.clone(),
                    trigger_label: target.label.clone(),
                    trigger_occurrence: Some(target.occurrence),
                    focused_path,
                    states: vec![result],
                });
                let mut successor = prefix.clone();
                successor.push(target.clone());
                prefixes.push(successor);
                interactions.len()
            } else {
                continue;
            };
            let edge = edge_candidate(state_index + 1, destination, &target);
            if !transitions
                .iter()
                .any(|existing| same_edge(existing, &edge))
            {
                transitions.push(edge);
            }
        }
        state_index += 1;
    }
    Ok(transitions)
}
