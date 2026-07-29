use crate::{browser::Browser, collector_browser::wait_replay_ready, model::Scenario};
use anyhow::Context;

pub async fn collect(
    browser: &mut Browser,
    url: &str,
    scenarios: &[Scenario],
) -> anyhow::Result<Vec<crate::model::Checkpoint>> {
    collect_with_mode(browser, url, scenarios, false).await
}

pub async fn collect_strict(
    browser: &mut Browser,
    url: &str,
    scenarios: &[Scenario],
) -> anyhow::Result<Vec<crate::model::Checkpoint>> {
    collect_with_mode(browser, url, scenarios, true).await
}

async fn collect_with_mode(
    browser: &mut Browser,
    url: &str,
    scenarios: &[Scenario],
    strict: bool,
) -> anyhow::Result<Vec<crate::model::Checkpoint>> {
    let started = std::time::Instant::now();
    let instrumentation = format!(
        "{};\n{};\nglobalThis.__recreateOracleCapture=({});",
        crate::determinism::INSTALL,
        crate::probe::INSTALL,
        crate::transition_probe::FUNCTION
    );
    browser.open_instrumented(url, &instrumentation).await?;
    wait_replay_ready(browser).await?;
    crate::collector_browser::resize(browser, 1280, 800).await?;
    let mut run = crate::collector_steps::Run::new(browser).await?;
    'scenarios: for scenario in scenarios {
        let scenario_started = std::time::Instant::now();
        let mut index = 0;
        for step in &scenario.steps {
            if let Err(error) = run
                .execute(scenario, step, &mut index)
                .await
                .with_context(|| format!("replaying {} step {index}: {step:?}", scenario.id))
            {
                if strict {
                    return Err(error);
                }
                run.fail(scenario, index, &error);
                break 'scenarios;
            }
        }
        if std::env::var_os("RECREATE_TIMING").is_some() {
            eprintln!(
                "oracle_scenario_ms={} id={}",
                scenario_started.elapsed().as_millis(),
                scenario.id
            );
        }
    }
    let checkpoints = run.finish();
    if std::env::var_os("RECREATE_TIMING").is_some() {
        eprintln!(
            "oracle_collect_ms={} checkpoints={}",
            started.elapsed().as_millis(),
            checkpoints.len()
        );
    }
    Ok(checkpoints)
}
