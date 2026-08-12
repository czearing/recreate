use super::css;
use crate::model::{Node, PageState, Rect, Specification, Styles, Viewport};

const PLAIN: &str = "html>body:nth-of-type(1)>div:nth-of-type(1)";
const OVERLAID: &str = "html>body:nth-of-type(1)>div:nth-of-type(2)";
const ORDERED: &str = "html>body:nth-of-type(1)>div:nth-of-type(3)";

/// A column flex container holding one absolutely positioned child pinned to its bottom
/// edge, written first in the markup. Per CSS Flexbox 4.1 that child is not a flex item and
/// does not participate in flex layout, so its `y` says nothing about the main axis. The
/// container is otherwise identical to a twin that omits the child.
#[test]
fn an_out_of_flow_child_does_not_reverse_its_containers_axis() {
    let output = css::build(&specification(), &Default::default());
    assert!(
        !output.css.contains("column-reverse") && !output.css.contains("row-reverse"),
        "an out-of-flow child reversed the emitted axis: {}",
        output.css
    );
    assert_eq!(
        output.css.matches("flex-direction:column;").count(),
        1,
        "the captured axis must survive, once, for the shared class: {}",
        output.css
    );
    assert_eq!(
        output.classes.get(PLAIN),
        output.classes.get(OVERLAID),
        "two containers identical in every captured declaration must share one class"
    );
}

/// `order` permutes flex items without changing `flex-direction`, so measured geometry
/// disagrees with the captured axis while the emitted `order` declarations already
/// reproduce the painted result. Emitting a reversal here applies the permutation twice.
#[test]
fn authored_order_is_replayed_rather_than_recompensated() {
    let output = css::build(&specification(), &Default::default());
    assert!(
        !output.css.contains("column-reverse"),
        "reordered items were compensated a second time on the axis: {}",
        output.css
    );
    assert_eq!(
        output.css.matches("order:").count(),
        3,
        "each item's captured order must reach the output: {}",
        output.css
    );
    assert_ne!(
        output.classes.get(ORDERED),
        None,
        "the reordered container must still be emitted"
    );
}

fn specification() -> Specification {
    Specification {
        schema_version: 1,
        requested_url: "https://example.com".into(),
        captured_url: "https://example.com".into(),
        states: vec![state()],
        interactions: Vec::new(),
        transitions: Vec::new(),
    }
}

fn state() -> PageState {
    let mut nodes = vec![node("html", None, Styles::new(), 0.0)];
    for path in [PLAIN, OVERLAID, ORDERED] {
        nodes.push(node(path, Some("html"), container(), 24.0));
    }
    // The badge is written first in the markup and painted at the container's bottom edge.
    nodes.push(node(
        &format!("{OVERLAID}>div:nth-of-type(1)"),
        Some(OVERLAID),
        out_of_flow(),
        159.0,
    ));
    for (index, top) in [33.0, 73.0, 113.0].into_iter().enumerate() {
        nodes.push(node(
            &format!("{PLAIN}>div:nth-of-type({})", index + 1),
            Some(PLAIN),
            item(None),
            top,
        ));
        nodes.push(node(
            &format!("{OVERLAID}>div:nth-of-type({})", index + 2),
            Some(OVERLAID),
            item(None),
            top,
        ));
        // Authored `order` paints these bottom to top while DOM order is unchanged.
        nodes.push(node(
            &format!("{ORDERED}>div:nth-of-type({})", index + 1),
            Some(ORDERED),
            item(Some(2 - index)),
            113.0 - top + 33.0,
        ));
    }
    PageState {
        url: "https://example.com".into(),
        title: "Example".into(),
        viewport: Viewport {
            width: 800,
            height: 600,
            dpr: 1.0,
        },
        dom: Default::default(),
        capture_blockers: Vec::new(),
        nodes,
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

fn container() -> Styles {
    let mut style = Styles::new();
    style.insert("display".into(), "flex".into());
    style.insert("flex-direction".into(), "column".into());
    style.insert("position".into(), "relative".into());
    style
}

fn item(order: Option<usize>) -> Styles {
    let mut style = Styles::new();
    style.insert("display".into(), "block".into());
    if let Some(order) = order {
        style.insert("order".into(), order.to_string());
    }
    style
}

fn out_of_flow() -> Styles {
    let mut style = Styles::new();
    style.insert("display".into(), "block".into());
    style.insert("position".into(), "absolute".into());
    style
}

fn node(path: &str, parent: Option<&str>, style: Styles, y: f64) -> Node {
    Node {
        disabled: false,
        rtl: false,
        path: path.into(),
        parent: parent.map(str::to_string),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 24.0,
            y,
            width: 96.0,
            height: 32.0,
        },
        style,
        before: None,
        after: None,
    }
}
