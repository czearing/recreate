use super::responsive;
use crate::model::Specification;
use std::collections::BTreeMap;

/// Authored declarations on html and body reach the output through this rule. The roots
/// carry their generated class like every other element, so this covers only what the
/// authored stylesheet said about them — naming a subset of properties here would silently
/// drop the rest, which is how a reset `background` on `body` went missing while the
/// `margin` beside it survived.
pub(super) fn root_reset(
    specification: &Specification,
    assets: &BTreeMap<String, String>,
) -> String {
    let Some(state) = specification.states.first() else {
        return String::new();
    };
    let mut css = String::new();
    for tag in ["html", "body"] {
        let Some(node) = state.nodes.iter().find(|node| node.tag == tag) else {
            continue;
        };
        let parent = node
            .parent
            .as_deref()
            .and_then(|path| state.nodes.iter().find(|other| other.path == path));
        let declarations = responsive::base_declarations(
            node,
            parent,
            &state.viewport,
            assets,
            &state.css_rules,
            false,
        );
        if !declarations.is_empty() {
            css.push_str(&format!("{tag}{{{declarations}}}\n"));
        }
    }
    css
}
