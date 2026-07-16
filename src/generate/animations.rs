use crate::model::Animation;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub fn append(animations: &[Animation], classes: &mut BTreeMap<String, String>, css: &mut String) {
    for (index, animation) in animations.iter().enumerate() {
        if animation.keyframes.len() < 2 {
            continue;
        }
        let name = format!("recreate{index}");
        let class = format!("a{index}");
        css.push_str(&format!("@keyframes {name}{{"));
        for (frame_index, frame) in animation.keyframes.iter().enumerate() {
            let offset = frame["offset"]
                .as_f64()
                .unwrap_or(frame_index as f64 / (animation.keyframes.len() - 1) as f64);
            css.push_str(&format!(
                "{}%{{{}}}",
                (offset * 100.0).round(),
                declarations(frame.as_object())
            ));
        }
        css.push_str("}\n");
        let duration = animation.timing["duration"].as_f64().unwrap_or(0.0);
        let delay = animation.timing["delay"].as_f64().unwrap_or(0.0);
        let easing = animation.timing["easing"].as_str().unwrap_or("linear");
        let iterations = animation.timing["iterations"].as_f64().unwrap_or(1.0);
        css.push_str(&format!(
            ".{class}{{animation:{name} {duration}ms {easing} {delay}ms {iterations};}}\n"
        ));
        classes
            .entry(animation.target.clone())
            .and_modify(|value| value.push_str(&format!(" {class}")));
    }
}

fn declarations(values: Option<&Map<String, Value>>) -> String {
    let Some(values) = values else {
        return String::new();
    };
    let mut output = String::new();
    let x = values.get("x").and_then(Value::as_f64);
    let y = values.get("y").and_then(Value::as_f64);
    if x.is_some() || y.is_some() {
        output.push_str(&format!(
            "translate:{}px {}px;",
            x.unwrap_or(0.0),
            y.unwrap_or(0.0)
        ));
    }
    for (key, value) in values {
        if ["offset", "easing", "composite", "computedOffset", "x", "y"].contains(&key.as_str()) {
            continue;
        }
        if let Some(value) = css_value(key, value) {
            output.push_str(&format!("{}:{value};", kebab(key)));
        }
    }
    output
}

fn css_value(key: &str, value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.into());
    }
    value.as_f64().map(|value| {
        if matches!(key, "width" | "height" | "left" | "top") {
            format!("{value}px")
        } else {
            value.to_string()
        }
    })
}

fn kebab(value: &str) -> String {
    value.chars().fold(String::new(), |mut result, character| {
        if character.is_uppercase() {
            result.push('-');
            result.extend(character.to_lowercase());
        } else {
            result.push(character);
        }
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn converts_geometry_frames_to_css() {
        let frame = json!({"offset":0.5,"x":10,"y":20,"opacity":"0.5"});
        let css = declarations(frame.as_object());
        assert!(css.contains("translate:10px 20px"));
        assert!(css.contains("opacity:0.5"));
    }
}
