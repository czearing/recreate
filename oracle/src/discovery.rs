use crate::{
    browser::Browser,
    checkpoint,
    collector_browser::{reload, resize, wait_rendered},
    discovery_obligations,
    model::{Checkpoint, Obligation, ObligationStatus, Scenario, Step, Viewport},
    probe, probe_discovery,
};

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
    browser.open_or_reuse(url).await?;
    wait_rendered(browser).await?;
    let script = browser
        .cdp
        .send(
            "Page.addScriptToEvaluateOnNewDocument",
            serde_json::json!({"source": probe::INSTALL}),
        )
        .await?;
    reload(browser).await?;
    resize(browser, width, height).await?;
    let value = browser
        .cdp
        .evaluate(&format!("({})({diagnostic})", probe_discovery::DISCOVER))
        .await?;
    let traced_checkpoint = checkpoint::capture(
        &mut browser.cdp,
        "trace-qualification",
        0,
        Viewport { width, height },
    )
    .await?;
    browser
        .cdp
        .send(
            "Page.removeScriptToEvaluateOnNewDocument",
            serde_json::json!({"identifier": script["identifier"]}),
        )
        .await?;
    let anchors = serde_json::from_value::<Vec<String>>(value["anchors"].clone())?;
    let persistent = serde_json::from_value::<Vec<bool>>(value["persistent"].clone())?;
    let interaction_scenarios = anchors
        .iter()
        .zip(persistent)
        .enumerate()
        .map(|(index, (anchor, persistent))| {
            let mut steps = vec![
                Step::Reset,
                Step::Hover {
                    anchor: anchor.clone(),
                },
                Step::Activate {
                    anchor: anchor.clone(),
                },
            ];
            if !persistent {
                steps.push(Step::Key {
                    key: "Escape".into(),
                });
            }
            Scenario {
                id: format!("interaction-{index}"),
                steps,
            }
        })
        .collect::<Vec<_>>();
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
    let mut obligations = anchors
        .iter()
        .enumerate()
        .map(|(index, _)| Obligation {
            id: format!("control:{index}"),
            kind: "trusted-input".into(),
            status: ObligationStatus::Qualified,
            scenarios: vec![format!("interaction-{index}")],
        })
        .collect::<Vec<_>>();
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
