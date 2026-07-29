use crate::{
    cdp::Cdp,
    digest,
    model::{Checkpoint, Domain, Viewport},
    transition_probe,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub async fn capture(cdp: &mut Cdp) -> anyhow::Result<Value> {
    cdp.evaluate(transition_probe::CAPTURE).await
}
pub fn state_digest(value: &Value) -> anyhow::Result<String> {
    crate::transition_state::digest(value, true)
}
pub fn reset_digest(value: &Value) -> anyhow::Result<String> {
    crate::transition_state::digest(value, false)
}
pub fn reset_state(value: &Value) -> Value {
    crate::transition_state::state(value, false)
}
pub fn affected_anchors(before: &Value, after: &Value, trigger: &str) -> BTreeSet<String> {
    let left = nodes(before);
    let right = nodes(after);
    let mut affected = BTreeSet::from([trigger.to_owned()]);
    for anchor in left.keys().chain(right.keys()).copied() {
        match (left.get(anchor), right.get(anchor)) {
            (Some(before), Some(after))
                if ["tag", "role", "name", "text", "state", "rect", "scroll"]
                    .into_iter()
                    .any(|key| before[key] != after[key]) =>
            {
                affected.insert(anchor.to_owned());
            }
            (None, Some(_)) | (Some(_), None) => {
                affected.insert(anchor.to_owned());
            }
            _ => {}
        }
    }
    affected
}

pub fn checkpoint(
    scenario: &str,
    step: usize,
    viewport: Viewport,
    before: &Value,
    after: &Value,
    _error_delta: usize,
) -> anyhow::Result<Checkpoint> {
    let left = nodes(before);
    let right = nodes(after);
    let anchors = left
        .keys()
        .chain(right.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let mut structure = Vec::new();
    let mut geometry = Vec::new();
    let mut style = Vec::new();
    for anchor in anchors {
        match (left.get(anchor), right.get(anchor)) {
            (None, Some(node)) => structure.push(json!({"anchor":anchor,"added":semantic(node)})),
            (Some(node), None) => structure.push(json!({"anchor":anchor,"removed":semantic(node)})),
            (Some(before), Some(after)) => {
                changed(
                    &mut structure,
                    anchor,
                    "semantic",
                    semantic(before),
                    semantic(after),
                );
                changed(
                    &mut geometry,
                    anchor,
                    "rect",
                    before["rect"].clone(),
                    after["rect"].clone(),
                );
                changed(
                    &mut geometry,
                    anchor,
                    "scroll",
                    before["scroll"].clone(),
                    after["scroll"].clone(),
                );
                changed(
                    &mut style,
                    anchor,
                    "style",
                    before["style"].clone(),
                    after["style"].clone(),
                );
                changed(
                    &mut structure,
                    anchor,
                    "asset",
                    before["asset"].clone(),
                    after["asset"].clone(),
                );
            }
            _ => {}
        }
    }
    let document = delta(&before["document"], &after["document"]);
    let observable = crate::transition_state::observable(
        !structure.is_empty() || !geometry.is_empty() || !style.is_empty(),
        &document,
        before,
        after,
    );
    let mut domains = BTreeMap::new();
    insert(&mut domains, "structure", json!({"transition":structure}))?;
    insert(
        &mut domains,
        "geometry",
        json!({"transition":geometry,"document":document}),
    )?;
    insert(&mut domains, "style", json!({"transition":style}))?;
    insert(
        &mut domains,
        "accessibility",
        json!({"focus_before":before["focus"],"focus_after":after["focus"],
            "transition":structure}),
    )?;
    insert(
        &mut domains,
        "interaction",
        json!({"action":crate::transition_state::action(&after["action"], observable)}),
    )?;
    insert(
        &mut domains,
        "async",
        json!({"browser_errors":after["action"]["errors"]}),
    )?;
    insert(
        &mut domains,
        "motion",
        json!({"before":before["animations"],"after":after["animations"]}),
    )?;
    Ok(Checkpoint {
        scenario: scenario.into(),
        step,
        viewport,
        domains,
    })
}

fn nodes(value: &Value) -> BTreeMap<&str, &Value> {
    value["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|node| Some((node["anchor"].as_str()?, node)))
        .collect()
}

fn semantic(node: &Value) -> Value {
    json!({"tag":node["tag"],"role":node["role"],"name":node["name"],
        "text":node["text"],"state":node["state"]})
}

fn changed(output: &mut Vec<Value>, anchor: &str, kind: &str, before: Value, after: Value) {
    if before != after {
        output.push(json!({"anchor":anchor,"kind":kind,"before":before,"after":after}));
    }
}

fn delta(before: &Value, after: &Value) -> Value {
    let before = before.as_array();
    let after = after.as_array();
    Value::Array(
        (0..before.map_or(0, Vec::len).max(after.map_or(0, Vec::len)))
            .map(|index| {
                json!(
                    after
                        .and_then(|values| values.get(index))
                        .and_then(Value::as_f64)
                        .unwrap_or_default()
                        - before
                            .and_then(|values| values.get(index))
                            .and_then(Value::as_f64)
                            .unwrap_or_default()
                )
            })
            .collect(),
    )
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
