use crate::model::{PageState, Specification};
use std::collections::{HashMap, HashSet};

pub fn with_baseline_css(mut state: PageState, baseline: &PageState) -> PageState {
    let mut rules = baseline.css_rules.clone();
    for rule in std::mem::take(&mut state.css_rules) {
        if !rules.contains(&rule) {
            rules.push(rule);
        }
    }
    state.css_rules = rules;
    // The divisions travel with the rules they key into, so a state indexed against the
    // baseline's sheets can still read a block the baseline recorded.
    for (block, shares) in &baseline.css_shorthands {
        state
            .css_shorthands
            .entry(block.clone())
            .or_insert_with(|| shares.clone());
    }
    state
}

pub fn fluid_height_paths(specification: &Specification) -> HashSet<String> {
    let mut heights = HashMap::<String, Vec<f64>>::new();
    let mut authored = HashSet::new();
    for state in &specification.states {
        let rules = super::authored_css::Index::new(state);
        for node in &state.nodes {
            heights
                .entry(node.path.clone())
                .or_default()
                .push(node.rect.height);
            if super::authored_css::has_property_indexed(node, &rules, "height") {
                authored.insert(node.path.clone());
            }
        }
    }
    heights
        .into_iter()
        .filter(|(path, values)| {
            !authored.contains(path)
                && !specification.states.iter().any(|state| {
                    state
                        .nodes
                        .iter()
                        .find(|node| &node.path == path)
                        .is_some_and(|node| {
                            node.style
                                .get("overflow")
                                .is_some_and(|value| value == "hidden")
                                || node
                                    .style
                                    .get("overflow-y")
                                    .is_some_and(|value| value == "hidden")
                                || node.style.contains_key("-webkit-line-clamp")
                        })
                })
                && values
                    .iter()
                    .skip(1)
                    .any(|value| (value - values[0]).abs() > 1.0)
        })
        .map(|(path, _)| path)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::model::{Attributes, Node, PageState, Rect};

    fn state(rules: &[&str], divisions: &[(&str, &[(&str, &str)])]) -> PageState {
        PageState {
            css_rules: rules.iter().map(|rule| (*rule).to_string()).collect(),
            css_shorthands: divisions
                .iter()
                .map(|(block, shares)| {
                    (
                        (*block).to_string(),
                        shares
                            .iter()
                            .map(|(name, share)| ((*name).to_string(), (*share).to_string()))
                            .collect(),
                    )
                })
                .collect(),
            ..Default::default()
        }
    }

    /// A later state is indexed against the baseline's sheets, because a state records only
    /// what its own interaction added. A division is how a block spelled as a shorthand is
    /// read at all, so a state that inherits the rule and not the division reads the rule as
    /// undecodable and publishes the conditional arm as the element's only value — the very
    /// defect, restored for every element an interaction touched.
    #[test]
    fn a_state_reads_a_division_the_baseline_recorded_for_a_rule_it_inherits() {
        let baseline = state(
            &[
                ".card { background: padding-box padding-box rgb(255, 0, 0); }",
                "@container (min-width: 500px){.card { background: rgb(0, 255, 0); }}",
            ],
            &[
                (
                    "background: padding-box padding-box rgb(255, 0, 0);",
                    &[("background-color", "rgb(255, 0, 0)")],
                ),
                (
                    "background: rgb(0, 255, 0);",
                    &[("background-color", "rgb(0, 255, 0)")],
                ),
            ],
        );
        let merged =
            super::with_baseline_css(state(&[".opened { display: block; }"], &[]), &baseline);

        let node = Node {
            attributes: Attributes::from([("class".to_string(), "card".to_string())]),
            rect: Rect {
                width: 10.0,
                height: 10.0,
                ..Default::default()
            },
            style: [("background-color".to_string(), "rgb(0, 255, 0)".to_string())]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let mut styles = node.style.clone();
        super::super::authored_conditions::restore_unconditional(
            &mut styles,
            &node,
            &super::super::authored_css::Index::new(&merged),
        );

        assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
    }
}
