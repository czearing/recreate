use super::scroll_state::{changed, resting, scrolls_document};
use crate::model::{DomNode, Node, PageState, Rect, Viewport};

pub(super) fn node(path: &str, tag: &str) -> Node {
    Node {
        path: path.into(),
        parent: Some("html>body".into()),
        tag: tag.into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 240.0,
        },
        style: Default::default(),
        before: None,
        after: None,
        disabled: false,
    }
}

pub(super) fn state(scrolls: &[(&str, f64, f64)]) -> PageState {
    let mut state = PageState {
        url: String::new(),
        title: String::new(),
        viewport: Viewport {
            width: 1920,
            height: 1080,
            dpr: 1.0,
        },
        nodes: vec![node("html", "html"), node("html>body", "body")],
        dom: Default::default(),
        capture_blockers: Vec::new(),
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    };
    for (path, left, top) in scrolls {
        state.nodes.push(node(path, "div"));
        state.dom.insert(
            (*path).into(),
            DomNode {
                scroll_left: *left,
                scroll_top: *top,
                ..DomNode::default()
            },
        );
    }
    state
}

/// The filed defect. An `overflow-y: hidden` panel is a scroll container per CSS Overflow 3
/// and scrolls from script; it must be credited exactly like the `auto` twin.
#[test]
fn credits_a_hidden_overflow_panel_the_same_as_an_auto_one() {
    let baseline = state(&[("auto", 0.0, 0.0), ("hidden", 0.0, 0.0)]);
    let after_auto = state(&[("auto", 0.0, 300.0), ("hidden", 0.0, 0.0)]);
    let after_hidden = state(&[("auto", 0.0, 0.0), ("hidden", 0.0, 300.0)]);
    let auto = changed(&baseline, &after_auto);
    let hidden = changed(&baseline, &after_hidden);
    assert_eq!(auto.len(), 1);
    assert_eq!(auto[0].path, "auto");
    assert_eq!(auto[0].top, 300);
    assert_eq!(hidden.len(), 1);
    assert_eq!(hidden[0].path, "hidden");
    assert_eq!(hidden[0].top, 300);
}

/// `overflow: clip` forbids scrolling through any mechanism, so it can never report a changed
/// offset and needs no predicate to exclude it. An unmoved element is never credited.
#[test]
fn never_credits_an_element_that_did_not_move() {
    let baseline = state(&[("clip", 0.0, 0.0), ("panel", 0.0, 0.0)]);
    let after = state(&[("clip", 0.0, 0.0), ("panel", 0.0, 300.0)]);
    let scrolled = changed(&baseline, &after);
    assert_eq!(scrolled.len(), 1);
    assert_eq!(scrolled[0].path, "panel");
}

/// The old ancestor walk stopped at the first container taller than 1.2x the viewport and
/// dropped the offset into the window slot. Height is not part of the question.
#[test]
fn credits_a_container_taller_than_the_viewport() {
    let baseline = state(&[("tall", 0.0, 0.0)]);
    let after = state(&[("tall", 0.0, 900.0)]);
    let scrolled = changed(&baseline, &after);
    assert_eq!(scrolled.len(), 1);
    assert_eq!(scrolled[0].top, 900);
}

/// The old anchor picked a single largest node per state, so only one scroll was ever
/// detected. Two panels scrolled by one action must both be credited.
#[test]
fn credits_every_panel_scrolled_in_one_state() {
    let baseline = state(&[("first", 0.0, 0.0), ("second", 0.0, 0.0)]);
    let after = state(&[("first", 0.0, 120.0), ("second", 0.0, 340.0)]);
    let scrolled = changed(&baseline, &after);
    assert_eq!(scrolled.len(), 2);
    assert_eq!(
        scrolled.iter().find(|s| s.path == "first").unwrap().top,
        120
    );
    assert_eq!(
        scrolled.iter().find(|s| s.path == "second").unwrap().top,
        340
    );
}

/// The genuine document scroll must keep working, and must be distinguishable so it reaches
/// the window slot rather than a `querySelector` call.
#[test]
fn identifies_the_document_scrolling_element() {
    let baseline = state(&[]);
    let mut after = state(&[]);
    after.dom.insert(
        "html".into(),
        DomNode {
            scroll_top: 640.0,
            ..DomNode::default()
        },
    );
    let scrolled = changed(&baseline, &after);
    assert_eq!(scrolled.len(), 1);
    assert!(scrolls_document(&after, scrolled[0].path));
    assert_eq!(scrolled[0].top, 640);
    assert!(!scrolls_document(&after, "html>body>main"));
}

/// A panel restored to the top is a real change and must be emitted, or the recreation keeps
/// the previous state's offset.
#[test]
fn credits_a_return_to_the_top_as_a_change() {
    let baseline = state(&[("panel", 0.0, 300.0)]);
    let after = state(&[("panel", 0.0, 0.0)]);
    let scrolled = changed(&baseline, &after);
    assert_eq!(scrolled.len(), 1);
    assert_eq!(scrolled[0].top, -300);
}

/// Both axes come from the same recorded pair, so they cannot disagree the way the two
/// hand-written allow-lists did.
#[test]
fn reports_both_axes_from_one_record() {
    let baseline = state(&[("pane", 0.0, 0.0)]);
    let after = state(&[("pane", 48.0, 300.0)]);
    let scrolled = changed(&baseline, &after);
    assert_eq!(scrolled.len(), 1);
    assert_eq!((scrolled[0].left, scrolled[0].top), (48, 300));
}

#[test]
fn resting_reports_offsets_held_at_capture() {
    let captured = state(&[("pane", 0.0, 160.0), ("still", 0.0, 0.0)]);
    let held = resting(&captured);
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].path, "pane");
    assert_eq!(held[0].top, 160);
}

/// Sub-pixel noise from device pixel ratio rounding must not be emitted as a scroll.
#[test]
fn ignores_sub_pixel_drift() {
    let baseline = state(&[("pane", 0.0, 0.0)]);
    let after = state(&[("pane", 0.4, 0.6)]);
    assert!(changed(&baseline, &after).is_empty());
}
