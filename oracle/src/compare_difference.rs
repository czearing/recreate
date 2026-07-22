use serde_json::Value;

pub fn between(left: &Value, right: &Value) -> (String, Value, Value) {
    walk(left, right, "$")
}

fn walk(left: &Value, right: &Value, path: &str) -> (String, Value, Value) {
    match (left, right) {
        (Value::Array(a), Value::Array(b)) => {
            for index in 0..a.len().max(b.len()) {
                if a.get(index) != b.get(index) {
                    return match (a.get(index), b.get(index)) {
                        (Some(x), Some(y)) => walk(x, y, &format!("{path}[{index}]")),
                        (x, y) => (
                            format!("{path}[{index}]"),
                            x.cloned().unwrap_or(Value::Null),
                            y.cloned().unwrap_or(Value::Null),
                        ),
                    };
                }
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            let keys = a
                .keys()
                .chain(b.keys())
                .collect::<std::collections::BTreeSet<_>>();
            for key in keys {
                if a.get(key) != b.get(key) {
                    return match (a.get(key), b.get(key)) {
                        (Some(x), Some(y)) => walk(x, y, &format!("{path}.{key}")),
                        (x, y) => (
                            format!("{path}.{key}"),
                            x.cloned().unwrap_or(Value::Null),
                            y.cloned().unwrap_or(Value::Null),
                        ),
                    };
                }
            }
        }
        _ => {}
    }
    (path.into(), left.clone(), right.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_shortest_path_and_leaf_values() {
        let left = serde_json::json!({"a": [{"x": 1}]});
        let right = serde_json::json!({"a": [{"x": 2}]});
        assert_eq!(
            between(&left, &right),
            ("$.a[0].x".into(), 1.into(), 2.into())
        );
    }
}
