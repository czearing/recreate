use crate::{
    capture_settle::note_curtain,
    model::{Node, PageState, Rect, Viewport},
};

/// The suspicion is recorded onto the artifact rather than raised, and the captured nodes
/// survive. Aborting here discarded every viewport already captured and wrote no files, so
/// the one record that could have adjudicated the guess was the thing it destroyed.
#[test]
fn records_a_blocking_overlay_without_discarding_the_capture() {
    let mut state = state();
    state.nodes.push(overlay("html>body>div"));

    note_curtain(&mut state);

    assert_eq!(
        state.capture_blockers,
        vec!["settled capture still contains a blocking overlay at html>body>div".to_string()]
    );
    assert_eq!(state.nodes.len(), 2, "captured nodes must survive the note");
}

#[test]
fn settled_content_without_a_blocking_overlay_is_not_noted() {
    let mut state = state();
    note_curtain(&mut state);
    assert!(state.capture_blockers.is_empty());
}

/// The verdict is a recorded fact about the element, not something re-derived from the
/// authored declarations. A node that satisfies every geometric and stacking clause but was
/// judged invisible by the engine carries alse, and nothing here may second-guess it.
#[test]
fn a_node_the_page_judged_invisible_is_not_noted_however_it_is_styled() {
    let mut state = state();
    let mut hidden = overlay("html>body>div");
    hidden.blocking_overlay = false;
    state.nodes.push(hidden);

    note_curtain(&mut state);

    assert!(state.capture_blockers.is_empty());
}

fn overlay(path: &str) -> Node {
    let mut overlay = node(path);
    overlay.rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    overlay.style.insert("position".into(), "absolute".into());
    overlay.style.insert("z-index".into(), "100".into());
    overlay.style.insert("pointer-events".into(), "auto".into());
    overlay.blocking_overlay = true;
    overlay
}

fn state() -> PageState {
    PageState {
        url: "https://example.test".into(),
        title: "Home".into(),
        viewport: Viewport {
            width: 1920,
            height: 1080,
            dpr: 1.0,
        },
        dom: Default::default(),
        capture_blockers: Vec::new(),
        nodes: vec![node("html>body>main")],
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

fn node(path: &str) -> Node {
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: path.into(),
        parent: None,
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        },
        style: Default::default(),
        before: None,
        after: None,
        ..Default::default()
    }
}
