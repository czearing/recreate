use crate::model::PageState;
use std::collections::BTreeSet;

pub fn properties(state: &PageState, path: &str) -> BTreeSet<String> {
    state
        .animations
        .iter()
        .filter(|animation| animation.target == path)
        .flat_map(|animation| &animation.keyframes)
        .filter_map(serde_json::Value::as_object)
        .flat_map(|frame| frame.keys())
        .filter(|property| {
            !matches!(
                property.as_str(),
                "composite" | "computedOffset" | "easing" | "offset" | "x" | "y"
            )
        })
        .map(|property| kebab(property))
        .collect()
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
