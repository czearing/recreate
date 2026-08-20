use super::*;

#[test]
fn identifies_dynamic_attributes() {
    assert!(dynamic_attribute("href"));
    assert!(dynamic_attribute("role"));
    assert!(dynamic_attribute("aria-expanded"));
    assert!(!dynamic_attribute("style"));
}

fn probe(path: &str, tag: &str) -> Node {
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: path.into(),
        parent: None,
        tag: tag.into(),
        text: String::new(),
        attributes: crate::model::Attributes::new(),
        rect: crate::model::Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        style: crate::model::Styles::new(),
        ..Default::default()
    }
}

#[test]
fn collapses_components_that_would_emit_the_same_body() {
    let first = probe("a", "span");
    let second = probe("b", "span");
    let third = probe("c", "div");
    let nodes = BTreeMap::from([
        ("a".to_string(), &first),
        ("b".to_string(), &second),
        ("c".to_string(), &third),
    ]);
    let merged = merge_identical_bodies(
        vec![
            (vec!["a".to_string()], 3),
            (vec!["b".to_string()], 2),
            (vec!["c".to_string()], 2),
        ],
        &nodes,
    );
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].0, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(merged[1].0, vec!["c".to_string()]);
}

/// Styling is a prop, so two groups of the same tag are one component however differently they
/// are styled. Keying the body on the class instead emitted one component per style bucket.
#[test]
fn a_differently_styled_group_is_not_a_separate_component() {
    let first = probe("a", "radialGradient");
    let second = probe("b", "radialGradient");
    let nodes = BTreeMap::from([("a".to_string(), &first), ("b".to_string(), &second)]);
    let merged = merge_identical_bodies(
        vec![(vec!["a".to_string()], 2), (vec!["b".to_string()], 2)],
        &nodes,
    );
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].0, vec!["a".to_string(), "b".to_string()]);
}
