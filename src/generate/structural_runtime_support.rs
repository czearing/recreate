use super::responsive_runtime_support::assert_exact_parity;
use crate::model::PageState;
use std::collections::BTreeMap;

pub fn assert_structural_parity(expected: &PageState, actual: &PageState) {
    assert_exact_parity(expected, actual);
    let actual: BTreeMap<_, _> = actual.nodes.iter().map(|node| (&node.path, node)).collect();
    for node in expected.nodes.iter().filter(|node| {
        !matches!(node.tag.as_str(), "html" | "body")
            && node.attributes.get("id").is_none_or(|id| id != "root")
    }) {
        let expected = relevant_attributes(&node.attributes);
        let actual = relevant_attributes(&actual[&node.path].attributes);
        assert_eq!(expected, actual, "attributes {}", node.path);
    }
}

fn relevant_attributes(attributes: &crate::model::Attributes) -> BTreeMap<&String, &String> {
    attributes
        .iter()
        .filter(|(name, _)| {
            !matches!(name.as_str(), "class" | "style") && !name.starts_with("data-recreate-")
        })
        .collect()
}
