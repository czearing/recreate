use crate::{
    browser::Browser,
    collector_browser::wait_rendered,
    discovery_interactions::Control,
    model::{Obligation, ObligationStatus, Scenario, Step},
    probe_discovery, replay,
};
use std::collections::{BTreeSet, VecDeque};

const MAX_CONTROLS: usize = 128;

pub async fn expand(
    browser: &mut Browser,
    url: &str,
    diagnostic: bool,
    controls: &[Control],
    scenarios: &mut Vec<Scenario>,
) -> anyhow::Result<Vec<Obligation>> {
    let mut known = controls
        .iter()
        .map(|control| control.anchor.clone())
        .collect::<BTreeSet<_>>();
    let mut obligations = Vec::new();
    let expandable = controls
        .iter()
        .filter(|control| control.popup || control.stateful)
        .map(|control| control.anchor.as_str())
        .collect::<BTreeSet<_>>();
    let mut queue = scenarios
        .iter()
        .filter(|scenario| {
            scenario.steps.iter().any(
                |step| matches!(step, Step::Activate { anchor } if expandable.contains(anchor.as_str())),
            )
        })
        .cloned()
        .collect::<VecDeque<_>>();
    while let Some(scenario) = queue.pop_front() {
        if reset(browser, url).await.is_err() {
            block(&scenario, scenarios, &mut obligations);
            continue;
        }
        let mut prefix = Vec::new();
        let mut replayable = true;
        for step in &scenario.steps {
            if execute(browser, step).await.is_err() {
                replayable = false;
                break;
            }
            prefix.push(step.clone());
            if !observes_successor(step) {
                continue;
            }
            for control in inspect(browser, diagnostic).await? {
                if !known.insert(control.anchor.clone()) {
                    continue;
                }
                anyhow::ensure!(
                    known.len() <= MAX_CONTROLS,
                    "action graph exceeded {MAX_CONTROLS} controls; certification is incomplete"
                );
                let successor = successor_scenario(&prefix, &control, scenarios.len());
                obligations.push(Obligation {
                    id: format!("control:{}", known.len() - 1),
                    kind: "trusted-input".into(),
                    status: ObligationStatus::Qualified,
                    scenarios: vec![successor.id.clone()],
                });
                if control.popup || control.stateful {
                    queue.push_back(successor.clone());
                }
                scenarios.push(successor);
            }
        }
        if !replayable {
            block(&scenario, scenarios, &mut obligations);
        }
    }

    fn block(
        scenario: &Scenario,
        scenarios: &mut Vec<Scenario>,
        obligations: &mut Vec<Obligation>,
    ) {
        scenarios.retain(|candidate| candidate.id != scenario.id);
        obligations.push(Obligation {
            id: format!("nondeterministic:{}", scenario.id),
            kind: "non-replayable-action".into(),
            status: ObligationStatus::Uncovered,
            scenarios: Vec::new(),
        });
    }
    reset(browser, url).await?;
    Ok(obligations)
}

async fn reset(browser: &mut Browser, url: &str) -> anyhow::Result<()> {
    browser
        .cdp
        .send("Page.navigate", serde_json::json!({"url":url}))
        .await?;
    for _ in 0..80 {
        let ready = browser
            .cdp
            .evaluate("({url:location.href,ready:document.readyState})")
            .await?;
        if ready["url"] == url && ready["ready"] == "complete" {
            return wait_rendered(browser).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    anyhow::bail!("source action reset did not restore its URL")
}

async fn inspect(browser: &mut Browser, diagnostic: bool) -> anyhow::Result<Vec<Control>> {
    let value = browser
        .cdp
        .evaluate(&format!("({})({diagnostic})", probe_discovery::DISCOVER))
        .await?;
    Ok(serde_json::from_value(value["controls"].clone())?)
}

async fn execute(browser: &mut Browser, step: &Step) -> anyhow::Result<()> {
    match step {
        Step::Reset => {}
        Step::Activate { anchor } => replay::activate(browser, anchor).await?,
        Step::Hover { anchor } => replay::hover(browser, anchor).await?,
        Step::Key { key } => replay::key(browser, key).await?,
        _ => {}
    }
    Ok(())
}

fn observes_successor(step: &Step) -> bool {
    matches!(step, Step::Activate { .. } | Step::Key { .. })
}

fn successor_scenario(prefix: &[Step], control: &Control, index: usize) -> Scenario {
    let mut steps = vec![Step::Reset];
    steps.extend(
        prefix
            .iter()
            .filter(|step| !matches!(step, Step::Reset))
            .cloned(),
    );
    steps.push(Step::Hover {
        anchor: control.anchor.clone(),
    });
    steps.push(Step::Activate {
        anchor: control.anchor.clone(),
    });
    steps.push(Step::Key {
        key: "Escape".into(),
    });
    Scenario {
        id: format!("successor-{index}"),
        steps,
    }
}
