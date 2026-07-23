use crate::{
    cdp::Cdp,
    digest,
    model::{Checkpoint, Domain, Viewport},
    network, snapshot_transfer,
};
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
];

pub async fn capture(
    cdp: &mut Cdp,
    scenario: &str,
    step: usize,
    viewport: Viewport,
) -> anyhow::Result<Checkpoint> {
    let started = std::time::Instant::now();
    let snapshot = snapshot_transfer::capture(cdp).await?;
    let snapshot_elapsed = started.elapsed();
    let accessibility = if snapshot["action"].is_null() {
        Some(
            cdp.send("Accessibility.getFullAXTree", json!({"depth": -1}))
                .await?,
        )
    } else {
        None
    };
    let accessibility_elapsed = started.elapsed();
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
            "ax": accessibility.as_ref().map(normalize_ax)
        }),
    )?;
    insert(
        &mut domains,
        "interaction",
        select(&snapshot, &["focus", "action"]),
    )?;
    let mut asynchronous = select(&snapshot, &["pending", "documentState"]);
    asynchronous["network"] = network::manifest(cdp).await?;
    asynchronous["browser_errors"] = serde_json::json!(cdp.error_count());
    insert(&mut domains, "async", asynchronous)?;
    insert(&mut domains, "motion", select(&snapshot, &["animations"]))?;
    if std::env::var_os("RECREATE_TIMING").is_some() {
        eprintln!(
            "oracle_checkpoint_ms snapshot={} accessibility={} total={}",
            snapshot_elapsed.as_millis(),
            (accessibility_elapsed - snapshot_elapsed).as_millis(),
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

pub fn failure(scenario: &str, step: usize, viewport: Viewport, error: &str) -> Checkpoint {
    let domains = DOMAINS
        .iter()
        .map(|name| {
            let value = if *name == "interaction" {
                serde_json::json!({"action":{"replay_error":error}})
            } else {
                serde_json::json!({"replay_error": error})
            };
            (
                (*name).into(),
                Domain {
                    digest: crate::digest::json(&value).unwrap_or_default(),
                    value,
                },
            )
        })
        .collect();
    Checkpoint {
        scenario: scenario.into(),
        step,
        viewport,
        domains,
    }
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
