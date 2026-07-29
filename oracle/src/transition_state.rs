use serde_json::Value;

pub(crate) fn digest(value: &Value, graph: bool) -> anyhow::Result<String> {
    crate::digest::json(&state(value, graph))
}

pub(crate) fn state(value: &Value, graph: bool) -> Value {
    let mut canonical = value.clone();
    if let Some(object) = canonical.as_object_mut() {
        object.remove("action");
        object.remove("animations");
        if let Some(document) = object.get_mut("document").and_then(Value::as_array_mut) {
            document.drain(..document.len().min(2));
        }
        if let Some(nodes) = object.get_mut("nodes").and_then(Value::as_array_mut) {
            if !graph {
                nodes.retain(|node| {
                    node["actionable"] == true
                        || node["scroll"]
                            .as_array()
                            .is_some_and(|scroll| scroll[0] != 0 || scroll[1] != 0)
                });
            }
            for node in nodes {
                if let Some(value) = node.as_object_mut() {
                    value.remove("text");
                    value.remove("style");
                    if !graph || value.get("actionable") != Some(&Value::Bool(true)) {
                        value.remove("rect");
                    }
                }
            }
        }
        if graph {
            object.remove("focus");
        }
    }
    canonical
}

pub(crate) fn action(value: &Value, observable: bool) -> Value {
    let mut action = value.clone();
    if let Some(object) = action.as_object_mut() {
        let first = object.get("first").and_then(Value::as_str);
        object.insert(
            "first".into(),
            Value::String(if observable && first != Some("none") {
                first.unwrap_or("immediate").into()
            } else if observable {
                "immediate".into()
            } else {
                "none".into()
            }),
        );
    }

    action
}

pub(crate) fn observable(changed: bool, document: &Value, before: &Value, after: &Value) -> bool {
    changed
        || document.as_array().is_some_and(|values| {
            values
                .iter()
                .any(|value| value.as_f64().unwrap_or_default() != 0.0)
        })
        || before["focus"] != after["focus"]
        || before["animations"] != after["animations"]
}

#[cfg(test)]
mod tests {
    use super::state;
    use serde_json::json;

    #[test]
    fn reset_identity_ignores_base_style_but_keeps_control_state() {
        let value = json!({"nodes":[{
            "actionable":true,
            "anchor":"button",
            "style":{"position":"relative"},
            "state":{"aria-selected":"true"},
            "scroll":[0,0,10,10]
        }]});
        let reset = state(&value, false);
        assert!(reset["nodes"][0].get("style").is_none());
        assert_eq!(reset["nodes"][0]["state"]["aria-selected"], "true");
    }
}
