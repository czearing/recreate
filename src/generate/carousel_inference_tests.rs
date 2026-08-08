//! A carousel is admitted by what the page declares itself to be, never by what it measures.
//!
//! Every geometric clause the inference once relied on — two sibling controls, exactly one of
//! them disabled, something overflowing horizontally nearby — is satisfied by an ordinary form
//! dialog. These tests hold both directions: the declared widget still binds, and the widget
//! that merely resembles one does not.

use super::carousel_inference::{EFFECT, javascript};
use crate::model::{DomNode, Node, PageState, Rect, Specification, Viewport};
use std::collections::BTreeMap;

/// The shape the filed defect fabricated from: a dialog whose action row holds a disabled
/// `Save` beside an enabled `Cancel`, above a panel that overflows horizontally. `container`
/// is what the enclosing element declares itself to be, and is the only variable these tests
/// move.
fn dialog(container: Option<&str>) -> Specification {
    let mut state = empty_state();
    let mut root = node("html>body>div", Some("html>body"), "div", 0.0, 340.0);
    if let Some(role) = container {
        root.attributes
            .insert("aria-roledescription".into(), role.into());
    }
    state.nodes = vec![
        root,
        node(
            "html>body>div>div:nth-of-type(1)",
            Some("html>body>div"),
            "div",
            10.0,
            200.0,
        ),
        node(
            "html>body>div>div:nth-of-type(1)>button:nth-of-type(1)",
            Some("html>body>div>div:nth-of-type(1)"),
            "button",
            10.0,
            60.0,
        ),
        node(
            "html>body>div>div:nth-of-type(1)>button:nth-of-type(2)",
            Some("html>body>div>div:nth-of-type(1)"),
            "button",
            10.0,
            60.0,
        ),
        node(
            "html>body>div>div:nth-of-type(2)",
            Some("html>body>div"),
            "div",
            60.0,
            200.0,
        ),
    ];
    state.nodes[2]
        .attributes
        .insert("disabled".into(), String::new());
    state.dom.insert(
        "html>body>div>div:nth-of-type(2)".into(),
        DomNode {
            scroll_width: 700.0,
            client_width: 200.0,
            ..Default::default()
        },
    );
    specification(state)
}

/// The fixture that was fabricated from. Nothing about it says "carousel", so the correct
/// output is silence: a static panel is the honest rendering of a widget never observed.
#[test]
fn refuses_controls_whose_container_declares_nothing() {
    assert_eq!(
        javascript(&dialog(None), false),
        "const inferredCarousel=null;"
    );
}

/// The narrowing must not become a deletion. The identical geometry, declared, still binds —
/// and still resolves the nearest overflowing target and its extent.
#[test]
fn infers_from_a_container_that_declares_itself_a_carousel() {
    let output = javascript(&dialog(Some("carousel")), false);
    assert!(output.contains("\"extent\":500"), "{output}");
    assert!(
        output.contains("\"previous\":\"html>body>div>div:nth-of-type(1)>button:nth-of-type(1)\""),
        "{output}"
    );
    assert!(
        output.contains("\"next\":\"html>body>div>div:nth-of-type(1)>button:nth-of-type(2)\""),
        "{output}"
    );
    assert!(
        output.contains("\"target\":\"html>body>div>div:nth-of-type(2)\""),
        "{output}"
    );
}

/// Presence of the attribute is not the signal; its value is. An element that declares itself
/// a toolbar has declared that it is not a carousel.
#[test]
fn refuses_a_container_that_declares_a_different_widget() {
    assert_eq!(
        javascript(&dialog(Some("toolbar")), false),
        "const inferredCarousel=null;"
    );
}

/// The disabled clause still applies inside a declared carousel: with nothing disabled there
/// is no resting position to advance from, so there is no pair to bind.
#[test]
fn refuses_a_declared_carousel_whose_controls_are_all_enabled() {
    let mut specification = dialog(Some("carousel"));
    specification.states[0].nodes[2]
        .attributes
        .remove("disabled");
    assert_eq!(
        javascript(&specification, false),
        "const inferredCarousel=null;"
    );
}

/// Overflow outside the declared container is another element's business. Binding to it would
/// let one page's carousel scroll an unrelated panel that merely sits below it.
#[test]
fn refuses_an_overflowing_target_outside_the_declared_container() {
    let mut specification = dialog(Some("carousel"));
    let state = &mut specification.states[0];
    state.dom.clear();
    state.nodes.push(node(
        "html>body>aside",
        Some("html>body"),
        "div",
        60.0,
        200.0,
    ));
    state.dom.insert(
        "html>body>aside".into(),
        DomNode {
            scroll_width: 700.0,
            client_width: 200.0,
            ..Default::default()
        },
    );
    assert_eq!(
        javascript(&specification, false),
        "const inferredCarousel=null;"
    );
}

/// A recorded carousel is reproduced by the captured path, so the guess must stay silent and
/// leave that path the only writer of these buttons.
#[test]
fn defers_to_a_captured_carousel() {
    assert_eq!(
        javascript(&dialog(Some("carousel")), true),
        "const inferredCarousel=null;"
    );
}

/// The rule shipped twice, and the browser copy was the weaker one: it scanned every element
/// for a disabled/enabled pair near overflow, keeping no constraint but the overflow itself,
/// and it ran precisely when the generator had declined to guess. The effect must therefore
/// choose nothing — every element it touches has to come from the decision already made.
#[test]
fn the_shipped_effect_never_chooses_a_carousel_itself() {
    let queries = EFFECT.match_indices("document.querySelector").count();
    assert_eq!(queries, 3, "{EFFECT}");
    for (index, _) in EFFECT.match_indices("document.querySelector") {
        let argument = &EFFECT[index..];
        let argument = &argument[argument.find('(').unwrap() + 1..];
        assert!(
            argument.starts_with("inferredCarousel."),
            "effect queries the document for something it was not given: {argument:.60}"
        );
    }
    assert!(!EFFECT.contains("querySelectorAll('body *')"), "{EFFECT}");
}

fn specification(state: PageState) -> Specification {
    Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![state],
        interactions: Vec::new(),
        transitions: Vec::new(),
    }
}

fn node(path: &str, parent: Option<&str>, tag: &str, y: f64, width: f64) -> Node {
    Node {
        disabled: false,
        path: path.into(),
        parent: parent.map(str::to_owned),
        tag: tag.into(),
        text: String::new(),
        attributes: BTreeMap::new(),
        rect: Rect {
            x: 0.0,
            y,
            width,
            height: 32.0,
        },
        style: BTreeMap::new(),
        before: None,
        after: None,
    }
}

fn empty_state() -> PageState {
    PageState {
        url: String::new(),
        title: String::new(),
        viewport: Viewport::default(),
        nodes: Vec::new(),
        dom: BTreeMap::new(),
        capture_blockers: Vec::new(),
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: BTreeMap::new(),
    }
}
