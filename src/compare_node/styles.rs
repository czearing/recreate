use crate::{
    compare_css_value,
    model::{Node, Pseudo, Styles},
};
use std::collections::BTreeSet;

pub(super) fn differences(left: &Node, right: &Node, animated: &BTreeSet<String>) -> Vec<String> {
    let same_geometry = super::same_rect(left, right);
    left.style
        .keys()
        .chain(right.style.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|key| {
            !(animated.contains(*key)
                || compare_css_value::equivalent(
                    left.style.get(*key).map(String::as_str),
                    right.style.get(*key).map(String::as_str),
                )
                || (same_geometry && compare_css_value::layout_property(key))
                || (!animated.is_empty() && compare_css_value::animation_property(key)))
        })
        .map(|key| {
            format!(
                "{key}={:?}/{:?}",
                left.style.get(key).map(String::as_str),
                right.style.get(key).map(String::as_str)
            )
        })
        .collect()
}

pub(super) fn same_pseudo(left: Option<&Pseudo>, right: Option<&Pseudo>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.content == right.content && differences_for(&left.style, &right.style).is_empty()
        }
        _ => false,
    }
}

fn differences_for(left: &Styles, right: &Styles) -> Vec<String> {
    left.keys()
        .chain(right.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|key| {
            !compare_css_value::equivalent(
                left.get(*key).map(String::as_str),
                right.get(*key).map(String::as_str),
            )
        })
        .cloned()
        .collect()
}
