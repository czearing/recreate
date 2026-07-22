use crate::{
    cdp::Cdp,
    digest,
    model::{Checkpoint, Domain, Viewport},
    network, probe,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const DOMAINS: &[&str] = &[
    "interaction",
    "async",
    "structure",
    "accessibility",
    "motion",
    "geometry",
    "style",
    "compositor",
];

pub async fn capture(
    cdp: &mut Cdp,
    scenario: &str,
    step: usize,
    viewport: Viewport,
) -> anyhow::Result<Checkpoint> {
    let started = std::time::Instant::now();
    let snapshot = cdp.evaluate(probe::SNAPSHOT).await?;
    let snapshot_elapsed = started.elapsed();
    let accessibility = cdp
        .send("Accessibility.getFullAXTree", json!({"depth": -1}))
        .await?;
    let accessibility_elapsed = started.elapsed();
    let screenshot_width = snapshot["visualViewport"][0]
        .as_f64()
        .unwrap_or_else(|| f64::from(viewport.width));
    let screenshot_height = snapshot["visualViewport"][1]
        .as_f64()
        .unwrap_or_else(|| f64::from(viewport.height))
        .min(
            snapshot["document"][1]
                .as_f64()
                .unwrap_or_else(|| f64::from(viewport.height)),
        )
        .max(1.0);
    let png = cdp
        .send(
            "Page.captureScreenshot",
            json!({
                "format": "png",
                "fromSurface": true,
                "captureBeyondViewport": false,
                "optimizeForSpeed": true,
                "clip": {
                    "x": 0,
                    "y": 0,
                    "width": screenshot_width,
                    "height": screenshot_height,
                    "scale": 1
                }
            }),
        )
        .await?;
    let screenshot_elapsed = started.elapsed();
    let encoded = STANDARD.decode(png["data"].as_str().unwrap_or_default())?;
    let mut decoder =
        zune_png::PngDecoder::new(zune_png::zune_core::bytestream::ZCursor::new(&encoded));
    let pixels = decoder
        .decode_raw()
        .map_err(|error| anyhow::anyhow!("decode compositor PNG: {error}"))?;
    let mut domains = BTreeMap::new();
    insert(
        &mut domains,
        "structure",
        serde_json::json!({
            "nodes": map_nodes(&snapshot, &["anchor", "tag", "role", "name", "text", "state"])?,
            "ambiguous": snapshot["ambiguous"]
        }),
    )?;
    insert(
        &mut domains,
        "geometry",
        serde_json::json!({
            "nodes": map_nodes(&snapshot, &["anchor", "rect", "boxes", "clips", "hit", "scroll"])?,
            "visual_viewport": snapshot["visualViewport"],
            "document": snapshot["document"]
        }),
    )?;
    insert(
        &mut domains,
        "style",
        map_nodes(&snapshot, &["anchor", "style"])?,
    )?;
    insert(
        &mut domains,
        "accessibility",
        serde_json::json!({
            "dom": map_nodes(&snapshot, &["anchor", "role", "name", "text", "state"])?,
            "ax": normalize_ax(&accessibility)
        }),
    )?;
    insert(&mut domains, "interaction", select(&snapshot, &["focus"]))?;
    let mut asynchronous = select(&snapshot, &["pending", "documentState"]);
    asynchronous["network"] = network::manifest(cdp).await?;
    asynchronous["browser_errors"] = serde_json::json!(cdp.error_count());
    insert(&mut domains, "async", asynchronous)?;
    insert(&mut domains, "motion", select(&snapshot, &["animations"]))?;
    let compositor = serde_json::json!({"sha256": digest::bytes(&pixels)});
    insert(&mut domains, "compositor", compositor)?;
    if std::env::var_os("RECREATE_TIMING").is_some() {
        eprintln!(
            "oracle_checkpoint_ms snapshot={} accessibility={} screenshot={} total={}",
            snapshot_elapsed.as_millis(),
            (accessibility_elapsed - snapshot_elapsed).as_millis(),
            (screenshot_elapsed - accessibility_elapsed).as_millis(),
            started.elapsed().as_millis()
        );
    }
    Ok(Checkpoint {
        scenario: scenario.into(),
        step,
        viewport,
        domains,
    })
}

fn normalize_ax(value: &Value) -> Value {
    Value::Array(
        value["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|node| node["ignored"] != true)
            .filter(|node| {
                node["role"]["value"] != "generic"
                    || node["name"]["value"]
                        .as_str()
                        .is_some_and(|name| !name.is_empty())
                    || node["description"]["value"]
                        .as_str()
                        .is_some_and(|description| !description.is_empty())
            })
            .map(|node| {
                serde_json::json!({
                    "role": node["role"]["value"],
                    "name": node["name"]["value"],
                    "description": node["description"]["value"],
                    "value": node["value"]["value"],
                    "properties": node["properties"].as_array().into_iter().flatten()
                        .filter_map(simple_ax_property).collect::<Vec<_>>()
                })
            })
            .collect(),
    )
}

fn simple_ax_property(value: &Value) -> Option<Value> {
    let inner = &value["value"]["value"];
    (value["name"] != "url"
        && matches!(inner, Value::Bool(_) | Value::Number(_) | Value::String(_)))
    .then(|| serde_json::json!({"name": value["name"], "value": inner}))
}

fn insert(domains: &mut BTreeMap<String, Domain>, name: &str, value: Value) -> anyhow::Result<()> {
    domains.insert(
        name.into(),
        Domain {
            digest: digest::json(&value)?,
            value,
        },
    );
    Ok(())
}

fn select(value: &Value, keys: &[&str]) -> Value {
    Value::Object(
        keys.iter()
            .map(|key| ((*key).into(), value[*key].clone()))
            .collect(),
    )
}

fn map_nodes(value: &Value, keys: &[&str]) -> anyhow::Result<Value> {
    let nodes = value["nodes"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("nodes missing"))?;
    Ok(Value::Array(
        nodes
            .iter()
            .map(|node| select(node, keys))
            .collect::<Vec<_>>(),
    ))
}
