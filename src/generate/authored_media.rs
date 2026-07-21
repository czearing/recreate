use crate::model::Node;
use std::collections::HashSet;

pub fn rules(node: &Node, generated_class: &str, rules: &[String]) -> Vec<String> {
    let classes = node
        .attributes
        .get("class")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
        .collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut seen = HashSet::new();
    for rule in rules {
        let Some((prefix, body)) = rule.split_once('{') else {
            continue;
        };
        let prefix = prefix.trim();
        if !prefix.starts_with("@media") {
            continue;
        }
        let condition = prefix.trim_start_matches("@media").trim();
        let body = body.trim_end().trim_end_matches('}');
        for child in body.split('}') {
            let Some((selector, declarations)) = child.split_once('{') else {
                continue;
            };
            let selector = selector.trim();
            if selector.contains(':')
                || !classes
                    .iter()
                    .any(|class| super::authored_css::directly_targets(selector, class))
            {
                continue;
            }
            let rule = format!(
                "@media {condition}{{.{generated_class}{{{}}}}}",
                declarations.trim()
            );
            if seen.insert(rule.clone()) {
                output.push(rule);
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Attributes, Rect};

    #[test]
    fn remaps_direct_authored_media_rules_to_generated_classes() {
        let node = Node {
            path: String::new(),
            parent: None,
            tag: "div".into(),
            text: String::new(),
            attributes: Attributes::from([("class".into(), "rail card".into())]),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            style: Default::default(),
            before: None,
            after: None,
        };
        let captured = vec![
            "@media (max-width: 1023px) { .rail { padding: 0 40px; } }".into(),
            "@media (max-width: 479px) { .card { grid-template-columns: 1fr; } }".into(),
            "@media (max-width: 479px) { .card:hover { color: red; } }".into(),
        ];

        let rules = rules(&node, "generated", &captured);

        assert_eq!(rules.len(), 2);
        assert!(rules[0].contains(".generated"));
        assert!(rules.iter().any(|rule| rule.contains("padding: 0 40px")));
        assert!(
            rules
                .iter()
                .any(|rule| rule.contains("grid-template-columns: 1fr"))
        );
    }
}
