use crate::{
    browser::Browser,
    checkpoint,
    collector_browser::{reload, resize, wait_rendered},
    discovery_obligations,
    model::{Checkpoint, Obligation, ObligationStatus, Scenario, Step, Viewport},
    probe, probe_discovery,
};
use anyhow::Context;

pub struct Discovery {
    pub scenarios: Vec<Scenario>,
    pub obligations: Vec<Obligation>,
    pub traced_checkpoint: Checkpoint,
}

pub async fn run(
    browser: &mut Browser,
    url: &str,
    viewport: (u32, u32),
    diagnostic: bool,
) -> anyhow::Result<Discovery> {
    let (width, height) = viewport;
    browser
        .open(url)
        .await
        .context("opening source discovery target")?;
    wait_rendered(browser)
        .await
        .context("waiting for source discovery target")?;
    let script = browser
        .cdp
        .send(
            "Page.addScriptToEvaluateOnNewDocument",
            serde_json::json!({"source": probe::INSTALL}),
        )
        .await
        .context("installing source discovery instrumentation")?;
    reload(browser)
        .await
        .context("reloading instrumented source discovery target")?;
    wait_rendered(browser)
        .await
        .context("waiting for hydrated source discovery target")?;
    resize(browser, width, height)
        .await
        .context("sizing source discovery target")?;
    let value = browser
        .cdp
        .evaluate(&format!("({})({diagnostic})", probe_discovery::DISCOVER))
        .await
        .context("discovering initial browser controls")?;
    let traced_checkpoint = checkpoint::capture(
        &mut browser.cdp,
        "trace-qualification",
        0,
        Viewport { width, height },
    )
    .await
    .context("capturing discovery qualification checkpoint")?;
    browser
        .cdp
        .send(
            "Page.removeScriptToEvaluateOnNewDocument",
            serde_json::json!({"identifier": script["identifier"]}),
        )
        .await?;
    let anchors = serde_json::from_value::<Vec<String>>(value["anchors"].clone())?;
    let controls = serde_json::from_value::<Vec<crate::discovery_interactions::Control>>(
        value["controls"].clone(),
    )?;
    let (interaction_scenarios, mut obligations) = crate::discovery_interactions::build(&controls);
    let mut scenarios = Vec::new();
    let boundaries = serde_json::from_value::<Vec<u32>>(value["boundaries"].clone())?;
    if !boundaries.is_empty() {
        let mut widths = boundaries.clone();
        widths.extend(boundaries.iter().rev().copied());
        scenarios.push(Scenario {
            id: "responsive-discovered".into(),
            steps: vec![Step::ResizePath {
                widths,
                height: 800,
            }],
        });
    }
    let frames = serde_json::from_value::<Vec<f64>>(value["motionFrames"].clone())?;
    if !frames.is_empty() && value["motionUnbounded"] != true {
        scenarios.push(Scenario {
            id: "motion-frames".into(),
            steps: frames
                .into_iter()
                .map(|milliseconds| Step::SeekAnimations {
                    milliseconds: milliseconds.round() as u32,
                })
                .collect(),
        });
    }
    scenarios.extend(interaction_scenarios);
    obligations.extend(
        crate::discovery_graph::expand(browser, url, diagnostic, &controls, &mut scenarios)
            .await
            .context("expanding browser-observed successor graph")?,
    );
    for token in value["opaque"].as_array().into_iter().flatten() {
        obligations.push(Obligation {
            id: format!("opaque:{}", token.as_str().unwrap_or("unknown")),
            kind: "opaque-surface".into(),
            status: ObligationStatus::Opaque,
            scenarios: Vec::new(),
        });
    }
    discovery_obligations::append(&value, anchors.is_empty(), &mut obligations);
    Ok(Discovery {
        scenarios,
        obligations,
        traced_checkpoint,
    })
}
