use crate::{
    browser::Browser,
    collector_browser::wait_interaction_ready,
    discovery_interactions::Control,
    model::{Obligation, ObligationStatus, Step},
    probe_discovery, replay,
};

pub(crate) async fn execute(
    browser: &mut Browser,
    step: &Step,
) -> anyhow::Result<serde_json::Value> {
    let state = match step {
        Step::Activate { anchor } => replay::activate(browser, anchor).await?,
        Step::Hover { anchor } => replay::hover(browser, anchor).await?,
        Step::Key { key } => replay::key(browser, key).await?,
        _ => crate::transition::capture(&mut browser.cdp).await?,
    };
    browser.refresh_network_fixture().await?;
    Ok(state)
}

pub(crate) async fn restore(
    browser: &mut Browser,
    url: &str,
    prefix: &[Step],
    controls: &[Control],
) -> anyhow::Result<()> {
    reset(browser, url, controls).await?;
    for step in prefix {
        execute(browser, step).await?;
    }
    Ok(())
}

pub(crate) async fn reset(
    browser: &mut Browser,
    url: &str,
    controls: &[Control],
) -> anyhow::Result<()> {
    browser.restore_storage_fixture().await?;
    browser
        .cdp
        .send("Page.navigate", serde_json::json!({"url":url}))
        .await?;
    let mut current = String::new();
    let mut state = String::new();
    for _ in 0..80 {
        browser.fulfill_network_fixture().await?;
        let ready = browser
            .cdp
            .evaluate("({url:location.href,ready:document.readyState})")
            .await?;
        current = ready["url"].as_str().unwrap_or_default().to_owned();
        state = ready["ready"].as_str().unwrap_or_default().to_owned();
        if current == url && state != "loading" {
            wait_interaction_ready(browser).await?;
            wait_controls(browser, controls).await?;
            browser.refresh_network_fixture().await?;
            return Ok(());
        }

        async fn wait_controls(browser: &mut Browser, expected: &[Control]) -> anyhow::Result<()> {
            for _ in 0..2400 {
                browser.fulfill_network_fixture().await?;
                let current = inspect(browser, true).await?;
                if expected
                    .iter()
                    .all(|control| current.iter().any(|item| item.anchor == control.anchor))
                {
                    return Ok(());
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            anyhow::bail!("source reset did not restore its discovered control set")
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    anyhow::bail!("source action reset expected {url}, restored {current} ({state})")
}

pub(crate) async fn inspect(
    browser: &mut Browser,
    diagnostic: bool,
) -> anyhow::Result<Vec<Control>> {
    let value = browser
        .cdp
        .evaluate(&format!("({})({diagnostic})", probe_discovery::DISCOVER))
        .await?;
    Ok(serde_json::from_value(value["controls"].clone())?)
}

pub(crate) fn prepare(step: &Step) -> Step {
    match step {
        Step::Activate { anchor } | Step::PrepareActivate { anchor } => Step::PrepareActivate {
            anchor: anchor.clone(),
        },
        Step::Hover { anchor } | Step::PrepareHover { anchor } => Step::PrepareHover {
            anchor: anchor.clone(),
        },
        step => step.clone(),
    }
}

pub(crate) fn action_kind(step: &Step) -> &'static str {
    match step {
        Step::Hover { .. } => "trusted-hover",
        Step::Activate { .. } => "trusted-activation",
        Step::Key { .. } => "trusted-key",
        _ => "browser-action",
    }
}

pub(crate) fn uncovered(kind: &str, detail: &str) -> Obligation {
    Obligation {
        id: format!("{kind}:{detail}"),
        kind: kind.into(),
        status: ObligationStatus::Uncovered,
        scenarios: Vec::new(),
    }
}

pub(crate) fn timed(started: std::time::Instant, states: usize, edges: usize) {
    if std::env::var_os("RECREATE_TIMING").is_some() {
        eprintln!(
            "oracle_graph_ms={} states={states} edges={edges}",
            started.elapsed().as_millis()
        );
    }
}
