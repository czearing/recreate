use super::tree::Components;
use crate::model::Specification;

/// The mount script assigns this to the existing `#root` element, which React takes over
/// and would otherwise render without its captured styles.
pub fn root_class(specification: &Specification, components: &Components) -> String {
    let Some(state) = specification.states.first() else {
        return String::new();
    };
    state
        .nodes
        .iter()
        .find(|node| {
            node.attributes
                .get("id")
                .is_some_and(|value| value == "root")
        })
        .and_then(|node| components.classes.get(&node.path))
        .cloned()
        .unwrap_or_default()
}
