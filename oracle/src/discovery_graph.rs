use crate::{
    browser::Browser,
    discovery_graph_support::{
        action_kind, execute, inspect, prepare, reset, restore, timed, uncovered,
    },
    discovery_interactions::Control,
    model::{Obligation, ObligationStatus, Scenario, Step},
    transition,
};
use std::collections::{BTreeSet, VecDeque};

const MAX_STATES: usize = 32;
const MAX_EDGES: usize = 256;

struct State {
    digest: String,
    prefix: Vec<Step>,
    controls: Vec<Control>,
}

pub async fn expand(
    browser: &mut Browser,
    _url: &str,
    diagnostic: bool,
    controls: &[Control],
    scenarios: &mut Vec<Scenario>,
) -> anyhow::Result<Vec<Obligation>> {
    let started = std::time::Instant::now();
    let restore_url = browser
        .cdp
        .evaluate("location.href")
        .await?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("source URL is unavailable"))?
        .to_owned();
    reset(browser, &restore_url, controls).await?;
    let initial = transition::capture(&mut browser.cdp).await?;
    let baseline_controls = controls.to_vec();
    let mut states = BTreeSet::from([transition::state_digest(&initial)?]);
    let mut queue = VecDeque::from([State {
        digest: transition::state_digest(&initial)?,
        prefix: Vec::new(),
        controls: controls.to_vec(),
    }]);
    let mut obligations = Vec::new();
    let mut edges = 0;
    let mut state_index = 0;
    while let Some(state) = queue.pop_front() {
        let hover_id = format!("hover-state-{state_index}");
        let stable_id = format!("activation-stable-{state_index}");
        state_index += 1;
        let mut hover_steps = vec![Step::Reset];
        hover_steps.extend(state.prefix.iter().map(prepare));
        let mut hover_count = 0;
        let mut stable_steps = Vec::new();
        let mut stable_count = 0;
        stable_steps.extend(state.prefix.iter().map(prepare));
        for control in state.controls {
            for action in actions(&control) {
                if edges >= MAX_EDGES {
                    obligations.push(uncovered("edge-budget", &state.digest));
                    timed(started, states.len(), edges);
                    reset(browser, &restore_url, &baseline_controls).await?;
                    return Ok(obligations);
                }
                restore(browser, &restore_url, &state.prefix, &baseline_controls).await?;
                let before = transition::capture(&mut browser.cdp).await?;
                let id = format!("edge-{edges}");
                let after = match execute(browser, &action).await {
                    Ok(after) => after,
                    Err(error) => {
                        obligations.push(Obligation {
                            id: format!("unreplayable:{}", obligations.len()),
                            kind: "trusted-input".into(),
                            status: ObligationStatus::Uncovered,
                            scenarios: Vec::new(),
                        });
                        eprintln!("uncovered source action {action:?}: {error:#}");
                        continue;
                    }
                };
                if after["action"]["unsafe"].as_u64().unwrap_or_default() > 0 {
                    obligations.push(uncovered("unsafe-side-effect", &format!("{action:?}")));
                    continue;
                }
                let digest = transition::state_digest(&after)?;
                let stable = matches!(action, Step::Activate { .. }) && digest == state.digest;
                let scenario = if matches!(action, Step::Hover { .. }) {
                    hover_steps.push(action.clone());
                    hover_count += 1;
                    hover_id.clone()
                } else if stable {
                    append_isolated(
                        &mut stable_steps,
                        &state.prefix,
                        action.clone(),
                        stable_count,
                    );
                    stable_count += 1;
                    stable_id.clone()
                } else {
                    let mut steps = vec![Step::Reset];
                    steps.extend(state.prefix.iter().map(prepare));
                    steps.push(action.clone());
                    scenarios.push(Scenario {
                        id: id.clone(),
                        steps,
                    });
                    id
                };
                obligations.push(Obligation {
                    id: format!("edge:{edges}"),
                    kind: action_kind(&action).into(),
                    status: ObligationStatus::Qualified,
                    scenarios: vec![scenario],
                });
                edges += 1;
                if edges % 10 == 0 {
                    timed(started, states.len(), edges);
                }
                if digest == state.digest || states.contains(&digest) {
                    continue;
                }
                if states.len() >= MAX_STATES {
                    obligations.push(uncovered("state-budget", &digest));
                    timed(started, states.len(), edges);
                    reset(browser, &restore_url, &baseline_controls).await?;
                    return Ok(obligations);
                }
                let affected = transition::affected_anchors(&before, &after, &control.anchor);
                let mut prefix = state.prefix.clone();
                prefix.push(action);
                let mut reproducible = true;
                for _ in 0..3 {
                    if restore(browser, &restore_url, &prefix, &baseline_controls)
                        .await
                        .is_err()
                    {
                        reproducible = false;
                        break;
                    }
                    let restored = transition::capture(&mut browser.cdp).await?;
                    if transition::state_digest(&restored)? != digest {
                        reproducible = false;
                        break;
                    }
                }
                if !reproducible {
                    obligations.push(uncovered("non-restorable-state", &digest));
                    continue;
                }
                states.insert(digest.clone());
                let controls = inspect(browser, diagnostic)
                    .await?
                    .into_iter()
                    .filter(|candidate| affected.contains(&candidate.anchor))
                    .collect::<Vec<_>>();
                queue.push_back(State {
                    digest,
                    prefix,
                    controls,
                });
            }
        }
        if stable_count > 0 {
            scenarios.push(Scenario {
                id: stable_id,
                steps: stable_steps,
            });
        }
        if hover_count > 0 {
            scenarios.push(Scenario {
                id: hover_id,
                steps: hover_steps,
            });
        }
    }
    reset(browser, &restore_url, &baseline_controls).await?;
    timed(started, states.len(), edges);
    Ok(obligations)
}

fn actions(control: &Control) -> [Step; 2] {
    [
        Step::Hover {
            anchor: control.anchor.clone(),
        },
        Step::Activate {
            anchor: control.anchor.clone(),
        },
    ]
}

fn append_isolated(steps: &mut Vec<Step>, prefix: &[Step], action: Step, _count: usize) {
    steps.push(Step::Reset);
    steps.extend(prefix.iter().map(prepare));
    steps.push(action);
}

#[cfg(test)]
mod tests {
    use super::{Step, append_isolated};

    #[test]
    fn stable_actions_are_isolated_by_reset() {
        let mut steps = Vec::new();
        append_isolated(&mut steps, &[], Step::Activate { anchor: "a".into() }, 0);
        append_isolated(&mut steps, &[], Step::Activate { anchor: "b".into() }, 1);
        assert!(matches!(steps[0], Step::Reset));
        assert!(matches!(steps[2], Step::Reset));
    }
}
