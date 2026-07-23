use crate::{browser::Browser, collector_browser::wait_rendered, model::Scenario};
use anyhow::Context;

pub async fn collect(
    browser: &mut Browser,
    url: &str,
    scenarios: &[Scenario],
) -> anyhow::Result<Vec<crate::model::Checkpoint>> {
    let started = std::time::Instant::now();
    browser
        .open_instrumented(url, crate::probe::INSTALL)
        .await?;
    wait_rendered(browser).await?;
    crate::collector_browser::resize(browser, 1280, 800).await?;
    let mut run = crate::collector_steps::Run::new(browser).await?;
    'scenarios: for scenario in scenarios {
        let mut index = 0;
        for step in &scenario.steps {
            if let Err(error) = run
                .execute(scenario, step, &mut index)
                .await
                .with_context(|| format!("replaying {} step {index}: {step:?}", scenario.id))
            {
                run.fail(scenario, index, &error);
                break 'scenarios;
            }
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
