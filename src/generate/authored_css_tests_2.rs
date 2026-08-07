use super::normalize;
use crate::model::{Node, Rect, Styles};

#[test]
fn removes_measured_height_from_growing_flex_items() {
    let mut node = Node {
        path: "form>div".into(),
        parent: Some("form".into()),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 34.0,
        },
        style: Styles::from([
            ("width".into(), "320px".into()),
            ("height".into(), "34px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "editor".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".editor { flex: 1 1 0%; min-height: 0; }".into()],
    );
    assert!(!node.style.contains_key("height"));
    assert_eq!(node.style["flex"], "1 1 0%");
    assert_eq!(node.style["min-height"], "0");
}

#[test]
fn keeps_used_height_for_percentage_sized_textareas() {
    let mut node = Node {
        path: "form>textarea".into(),
        parent: Some("form".into()),
        tag: "textarea".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 48.0,
        },
        style: Styles::from([
            ("width".into(), "320px".into()),
            ("height".into(), "48px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "editor".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".editor { flex: 1 1 0%; width: 100%; height: 100%; }".into()],
    );
    assert_eq!(node.style["width"], "100%");
    assert_eq!(node.style["height"], "48px");
}

#[test]
fn removes_sampled_height_from_intrinsic_flex_cards() {
    let mut node = Node {
        path: "article>button".into(),
        parent: Some("article".into()),
        tag: "button".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 369.0,
            height: 245.0,
        },
        style: Styles::from([
            ("display".into(), "flex".into()),
            ("height".into(), "245px".into()),
            ("overflow".into(), "hidden".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "task-card".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".task-card { display: flex; min-height: 132px; overflow: hidden; }".into()],
    );
    assert!(!node.style.contains_key("height"));
    assert_eq!(node.style["min-height"], "132px");
    assert_eq!(node.style["overflow"], "hidden");
}

/// A column section that stacks a heading above a wrapping card grid is sized
/// by its content. Freezing the height sampled at the capture viewport keeps
/// the single-row height, so once the grid wraps the extra row overflows the
/// section and every section below it moves up by that row's height.
#[test]
fn removes_sampled_height_from_a_content_sized_flex_section() {
    let mut node = Node {
        path: "main>section".into(),
        parent: Some("main".into()),
        tag: "section".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 980.0,
            height: 204.0,
        },
        style: Styles::from([
            ("display".into(), "flex".into()),
            ("flex-direction".into(), "column".into()),
            ("height".into(), "204px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes
        .insert("class".into(), "curatedSection".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".curatedSection { display: flex; flex-direction: column; gap: 12px; }".into()],
    );
    assert!(!node.style.contains_key("height"));
    assert_eq!(node.style["display"], "flex");
}

/// An authored minimum height is a floor the author expects content to grow past. A card
/// whose body text wraps onto an extra line at a narrower width has to get taller; freezing
/// the sampled height instead spills the text past the card's own border.
#[test]
fn removes_sampled_height_from_a_block_card_with_an_authored_minimum() {
    let mut node = Node {
        path: "div>div".into(),
        parent: Some("div".into()),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 229.0,
            height: 168.0,
        },
        style: Styles::from([
            ("display".into(), "block".into()),
            ("height".into(), "168px".into()),
            ("min-height".into(), "152px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "card".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".card { display: block; min-height: var(--card-min-height); }".into()],
    );
    assert!(!node.style.contains_key("height"));
}

/// The authored declaration map drops custom-property values, so a header sized
/// by `height: var(--app-bar-height)` looks unauthored. Dropping its sampled
/// height collapses the bar to its content and lifts the whole page under it.
#[test]
fn keeps_the_height_a_flex_container_authored_through_a_variable() {
    let mut node = Node {
        path: "body>header".into(),
        parent: Some("body".into()),
        tag: "header".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 48.0,
        },
        style: Styles::from([
            ("display".into(), "flex".into()),
            ("height".into(), "48px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "appBar".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".appBar { display: flex; height: var(--app-bar-height); }".into()],
    );
    assert_eq!(node.style["height"], "48px");
}

/// An author who sets an explicit height means it, so the sampled height has to
/// survive rather than collapsing the box to its content.
#[test]
fn keeps_the_height_a_flex_container_authored() {
    let mut node = Node {
        path: "main>section".into(),
        parent: Some("main".into()),
        tag: "section".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 980.0,
            height: 204.0,
        },
        style: Styles::from([
            ("display".into(), "flex".into()),
            ("height".into(), "204px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "banner".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".banner { display: flex; height: 204px; }".into()],
    );
    assert_eq!(node.style["height"], "204px");
}

#[test]
fn removes_resolved_grid_rows_without_authored_tracks() {
    let mut node = Node {
        path: "main>section".into(),
        parent: Some("main".into()),
        tag: "section".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 389.0,
            height: 536.0,
        },
        style: Styles::from([
            ("display".into(), "grid".into()),
            ("grid-template-columns".into(), "389px".into()),
            ("grid-template-rows".into(), "168px 164px 164px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "cards".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".cards { display: grid; grid-template-columns: 1fr; gap: 20px; }".into()],
    );

    assert!(!node.style.contains_key("grid-template-rows"));
    assert_eq!(node.style["grid-template-columns"], "1fr");
}

#[test]
fn rejects_authored_layout_values_from_inactive_media_rules() {
    let mut node = Node {
        path: "header".into(),
        parent: None,
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 40.0,
        },
        style: Styles::from([
            ("display".into(), "flex".into()),
            ("flex-direction".into(), "row".into()),
            ("gap".into(), "normal".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "header".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".header { flex-direction: column; gap: 4px; }".into()],
    );
    assert_eq!(node.style["flex-direction"], "row");
    assert_eq!(node.style["gap"], "normal");
}

/// `all: unset` names no value, so it must never be emitted. A source that resets a
/// control and is then given real padding by a component library resolves to that
/// padding; emitting the keyword instead overrides the correct value with zero and
/// silently shrinks every such control.
#[test]
fn ignores_cascade_keywords_when_reading_authored_values() {
    let mut node = Node {
        path: "div>button".into(),
        parent: Some("div".into()),
        tag: "button".into(),
        text: "All".into(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 37.0,
            height: 32.0,
        },
        style: Styles::from([
            ("padding-left".into(), "10px".into()),
            ("padding-right".into(), "10px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes
        .insert("class".into(), "filterButton".into());
    let rules = [
        ".filterButton { all: unset; }".to_string(),
        ".filterButton { padding: 6px 10px; }".to_string(),
    ];
    let index = crate::generate::authored_css_index::Index::new(&rules);
    assert_eq!(
        index.authored_value(&node, "padding"),
        Some("6px 10px".into())
    );
    for keyword in ["unset", "initial", "inherit", "revert", "revert-layer"] {
        let rules = [format!(".filterButton {{ padding: {keyword}; }}")];
        let only = crate::generate::authored_css_index::Index::new(&rules);
        assert_eq!(only.authored_value(&node, "padding"), None, "{keyword}");
    }
}

/// `:where()` and `:is()` match on structure, not state, so a rule using either applies
/// in the base state. Fluent defines a card's padding only on `.root:where(.size-medium)`,
/// so discarding the rule leaves the padding variable empty and collapses the card to
/// zero padding. A real state pseudo-class must still be discarded.
#[test]
fn reads_authored_values_from_static_pseudo_class_selectors() {
    let mut node = Node {
        path: "div>div".into(),
        parent: Some("div".into()),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 230.0,
            height: 160.0,
        },
        style: Styles::from([("padding".into(), "12px".into())]),
        before: None,
        after: None,
    };
    node.attributes
        .insert("class".into(), "root sizeMedium".into());
    let rules = [".root:where(.sizeMedium) { padding: 12px; }".to_string()];
    let index = crate::generate::authored_css_index::Index::new(&rules);
    assert_eq!(index.authored_value(&node, "padding"), Some("12px".into()));

    let nested = [".root:is(.sizeMedium:where(.root)) { padding: 12px; }".to_string()];
    let nested = crate::generate::authored_css_index::Index::new(&nested);
    assert_eq!(nested.authored_value(&node, "padding"), Some("12px".into()));

    let hovered = [".root:hover { padding: 12px; }".to_string()];
    let hovered = crate::generate::authored_css_index::Index::new(&hovered);
    assert_eq!(hovered.authored_value(&node, "padding"), None);
}

/// Two rules can declare the same property, and this index models neither `@layer`
/// order nor specificity. The captured computed value decides which one won: a card
/// that computes to 12px of padding did not take the `padding: 0px` a later rule
/// wrote, so emitting that literal would replace correct geometry with a loser.
#[test]
fn ignores_authored_values_the_captured_style_contradicts() {
    let mut node = Node {
        path: "div>div".into(),
        parent: Some("div".into()),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 230.0,
            height: 160.0,
        },
        style: Styles::from([("padding".into(), "12px".into())]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "root card".into());
    let rules = [
        ".root { padding: var(--component-card-padding); }".to_string(),
        ".card { padding: 0px; }".to_string(),
    ];
    let index = crate::generate::authored_css_index::Index::new(&rules);
    assert_eq!(index.authored_value(&node, "padding"), Some("12px".into()));

    let agreeing = [
        ".root { padding: var(--component-card-padding); }".to_string(),
        ".card { padding: 12px; }".to_string(),
    ];
    let agreeing = crate::generate::authored_css_index::Index::new(&agreeing);
    assert_eq!(
        agreeing.authored_value(&node, "padding"),
        Some("12px".into())
    );

    let losing_only = [".card { padding: 0px; }".to_string()];
    let losing_only = crate::generate::authored_css_index::Index::new(&losing_only);
    assert_eq!(
        losing_only.authored_value(&node, "padding"),
        Some("12px".into())
    );
    assert_eq!(
        losing_only.declarations(&node).get("padding"),
        Some(&"12px".to_string())
    );
}
