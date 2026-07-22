use crate::{
    capture_startup::ensure_settled,
    model::{Node, PageState, Rect, Viewport},
};

#[test]
fn rejects_blocking_overlay_in_settled_capture() {
    let mut state = state();
    let mut overlay = node("html>body>div");
    overlay.rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    overlay.style.insert("position".into(), "absolute".into());
    overlay.style.insert("z-index".into(), "100".into());
    overlay.style.insert("pointer-events".into(), "auto".into());
    state.nodes.push(overlay);

    assert!(ensure_settled(&state).is_err());
}

#[test]
fn accepts_settled_content_without_blocking_overlay() {
    assert!(ensure_settled(&state()).is_ok());
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
    }
}
