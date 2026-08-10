use super::{
    interactions_activate::{aim, fire},
    interactions_discovery::discover_transitions,
    interactions_evidence::{
        InteractionAlias, action_family, deduplicate, responsive_baselines, surface_backed,
    },
    interactions_graph::edge,
    interactions_runtime::{close, restore, settle},
    interactions_scope::{begin_scope, take_scope},
    interactions_scripts::{CANDIDATES, Candidate, PREFLIGHT},
};
use crate::{
    capture,
    cdp::Cdp,
    interaction_state,
    interactions_input::focused_path,
    model::{Interaction, InteractionTransition, PageState},
};
use anyhow::Result;

pub struct CapturedGraph {
    pub interactions: Vec<Interaction>,
    pub transitions: Vec<InteractionTransition>,
}

pub async fn capture_graph(cdp: &mut Cdp, baselines: &[PageState]) -> Result<CapturedGraph> {
    let (mut interactions, aliases) = capture_states(cdp, baselines).await?;
    deduplicate(&mut interactions);
    let base_states = interactions.len();
    let mut transitions = match discover_transitions(cdp, baselines, &mut interactions).await {
        Ok(transitions) => transitions,
        Err(error) => {
            if recreate_browser::transport_lost(&error) {
                return Err(error.context("lost the browser during transition expansion"));
            }
            eprintln!("stopped optional transition expansion: {error:#}");
            interactions.truncate(base_states);
            interactions
                .iter()
                .enumerate()
                .map(|(index, interaction)| edge(0, index + 1, interaction))
                .collect()
        }
    };
    for alias in aliases {
        let Some(destination) = interactions.iter().position(|interaction| {
            interaction.trigger_path == alias.template_path
                && interaction.trigger_tag == alias.template_tag
                && interaction.trigger_label == alias.template_label
        }) else {
            continue;
        };
        transitions.push(super::interactions_graph::edge_candidate(
            0,
            destination + 1,
            &alias.candidate,
        ));
    }
    Ok(CapturedGraph {
        interactions,
        transitions,
    })
}

pub(super) async fn capture_states(
    cdp: &mut Cdp,
    baselines: &[PageState],
) -> Result<(Vec<Interaction>, Vec<InteractionAlias>)> {
    let Some(first) = baselines.first() else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut initial = Some(restore(cdp, first, false).await?);
    let candidates: Vec<Candidate> = serde_json::from_value(cdp.evaluate(CANDIDATES).await?)?;
    let mut interactions = Vec::new();
    let mut aliases = Vec::new();
    for candidate in &candidates {
        cdp.take_events();
        if candidate.disabled || candidate.navigates {
            continue;
        }
        if candidate.occurrence > 0
            && let Some(shared) = interactions
                .iter_mut()
                .find(|interaction: &&mut Interaction| {
                    interaction
                        .trigger_label
                        .eq_ignore_ascii_case(&candidate.label)
                        && interaction.trigger_tag == candidate.tag
                        && surface_backed(interaction, baselines)
                })
        {
            shared.trigger_occurrence = None;
            continue;
        }
        if let Some(family) = action_family(&candidate.label)
            && let Some(template) = interactions.iter().find(|interaction| {
                interaction.trigger_tag == candidate.tag
                    && action_family(&interaction.trigger_label) == Some(family)
            })
        {
            aliases.push(InteractionAlias {
                candidate: candidate.clone(),
                template_path: template.trigger_path.clone(),
                template_tag: template.trigger_tag.clone(),
                template_label: template.trigger_label.clone(),
            });
            continue;
        }
        let candidate_started = std::time::Instant::now();
        let fresh = match initial.take() {
            Some(state) => state,
            None => match restore(cdp, first, false).await {
                Ok(state) => state,
                Err(error) => {
                    if recreate_browser::transport_lost(&error) {
                        return Err(error.context("lost the browser during interaction capture"));
                    }
                    eprintln!("stopped base interaction capture: {error:#}");
                    break;
                }
            },
        };
        begin_scope(cdp, &candidate.path).await?;
        let aimed = aim(cdp, candidate).await?;
        let Some(before) = fire(cdp, aimed).await? else {
            let _ = take_scope(cdp).await?;
            initial = Some(fresh);
            continue;
        };
        let settled = settle(cdp, candidate.uses_text_entry()).await?;
        let affected = take_scope(cdp).await?;
        if cdp.evaluate(PREFLIGHT).await? == before {
            // The page is provably where it was, so it does not have to be driven back and the
            // snapshot of it at rest is still the one the next candidate needs.
            initial = Some(fresh);
            continue;
        }
        let focused_path = focused_path(cdp).await?;
        if cdp.evaluate("location.href").await?.as_str() != Some(first.url.as_str()) {
            continue;
        }
        let mut state = capture::read_interaction_state(cdp, first.viewport.clone()).await?;
        crate::interaction_rebase::causal(&mut state, &fresh, &affected);
        close(cdp, candidate).await?;
        crate::interaction_rebase::unchanged(&mut state, &fresh, first);
        interaction_state::compact(&mut state, &fresh, settled);
        let responsive = responsive_baselines(
            candidate.uses_text_entry(),
            candidate.state_control,
            &state,
            &fresh,
            &candidate.path,
            &candidate.label,
            baselines,
        );
        let mut states = vec![state];
        for baseline in responsive {
            let fresh = restore(cdp, baseline, false).await?;
            begin_scope(cdp, &candidate.path).await?;
            let aimed = aim(cdp, candidate).await?;
            let Some(before) = fire(cdp, aimed).await? else {
                let _ = take_scope(cdp).await?;
                continue;
            };

            let settled = settle(cdp, candidate.uses_text_entry()).await?;
            let affected = take_scope(cdp).await?;
            let mut state = capture::read_interaction_state(cdp, baseline.viewport.clone()).await?;
            if cdp.evaluate(PREFLIGHT).await? != before {
                crate::interaction_rebase::causal(&mut state, &fresh, &affected);
                crate::interaction_rebase::unchanged(&mut state, &fresh, baseline);
                interaction_state::compact(&mut state, &fresh, settled);
                states.push(state);
                close(cdp, candidate).await?;
            }
        }
        eprintln!(
            "captured interaction {:?} in {:.2}s",
            candidate.label,
            candidate_started.elapsed().as_secs_f64()
        );
        interactions.push(Interaction {
            trigger_path: candidate.path.clone(),
            trigger_tag: candidate.tag.clone(),
            trigger_label: candidate.label.clone(),
            trigger_occurrence: Some(candidate.occurrence),
            focused_path,
            states,
        });
        cdp.take_events();
    }
    Ok((interactions, aliases))
}
