use super::tree::Components;
use crate::model::Specification;

pub fn classes(specification: &Specification, components: &Components) -> (String, String, String) {
    let Some(state) = specification.states.first() else {
        return Default::default();
    };
    let class = |tag: &str| {
        state
            .nodes
            .iter()
            .find(|node| node.tag == tag)
            .and_then(|node| components.classes.get(&node.path))
            .cloned()
            .unwrap_or_default()
    };
    let root = state
        .nodes
        .iter()
        .find(|node| {
            node.attributes
                .get("id")
                .is_some_and(|value| value == "root")
        })
        .and_then(|node| components.classes.get(&node.path))
        .cloned()
        .unwrap_or_default();
    (class("html"), class("body"), root)
}
