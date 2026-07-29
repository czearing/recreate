use crate::{collector_steps::Run, model::Scenario, transition};

pub(crate) async fn push(
    run: &mut Run<'_>,
    scenario: &Scenario,
    step: usize,
    before: &serde_json::Value,
    after: serde_json::Value,
) -> anyhow::Result<()> {
    run.checkpoints.push(transition::checkpoint(
        &scenario.id,
        step,
        run.viewport.clone(),
        before,
        &after,
        0,
    )?);
    update(run, after)?;
    Ok(())
}

pub(crate) async fn before(run: &mut Run<'_>) -> anyhow::Result<serde_json::Value> {
    if run.transition_state.is_none() {
        let captured = transition::capture(&mut run.browser.cdp).await?;
        update(run, captured)?;
    }
    Ok(run.transition_state.clone().unwrap())
}

pub(crate) async fn refresh(run: &mut Run<'_>) -> anyhow::Result<()> {
    let captured = transition::capture(&mut run.browser.cdp).await?;
    update(run, captured)
}

pub(crate) fn update(run: &mut Run<'_>, state: serde_json::Value) -> anyhow::Result<()> {
    run.clean = transition::reset_digest(&state)? == run.baseline_digest;
    run.transition_state = Some(state);
    Ok(())
}

pub(crate) fn reset_error(
    baseline: &serde_json::Value,
    current: &serde_json::Value,
) -> anyhow::Error {
    let (path, expected, actual) = crate::compare_difference::between(
        &transition::reset_state(baseline),
        &transition::reset_state(current),
    );
    anyhow::anyhow!(
        "reset did not restore canonical state at {path}: expected={expected} actual={actual}"
    )
}

pub(crate) async fn wait_reset(run: &mut Run<'_>) -> anyhow::Result<()> {
    let mut current = serde_json::Value::Null;
    let expected = action_anchors(&run.baseline_transition);
    let mut previous = String::new();
    let mut stable = 0;
    for _ in 0..2400 {
        current = transition::capture(&mut run.browser.cdp).await?;
        let digest = transition::reset_digest(&current)?;
        if digest == run.baseline_digest {
            update(run, current)?;
            return Ok(());
        }
        if expected.is_subset(&action_anchors(&current)) {
            stable = if digest == previous { stable + 1 } else { 0 };
            if stable >= 2 {
                break;
            }
            previous = digest;
        } else {
            stable = 0;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    Err(reset_error(&run.baseline_transition, &current))
}

fn action_anchors(value: &serde_json::Value) -> std::collections::BTreeSet<&str> {
    value["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|node| node["actionable"] == true)
        .filter_map(|node| node["anchor"].as_str())
        .collect()
}
