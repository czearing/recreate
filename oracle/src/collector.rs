use crate::{browser::Browser, collector_browser::reload, model::Scenario};

pub async fn collect(
    browser: &mut Browser,
    url: &str,
    scenarios: &[Scenario],
) -> anyhow::Result<Vec<crate::model::Checkpoint>> {
    browser.open(url).await?;
    reload(browser).await?;
    let mut run = crate::collector_steps::Run::new(browser);
    for scenario in scenarios {
        let mut index = 0;
        for step in &scenario.steps {
            run.execute(scenario, step, &mut index).await?;
        }
    }
    Ok(run.finish())
}
