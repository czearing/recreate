//! Two panes measured identically at capture must be emitted identically, whatever keyword
//! produced the gutter - and a pane that reserved nothing must be left alone.

use super::*;
use crate::model::{Attributes, Rect};

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    dpr: 1.0,
};

/// A pane from the subject scene: authored `width: 300px` with a 1px right border, so the
/// engine resolved `width` to 290px after taking out a 10px scrollbar. `scrollbar_gutter` is
/// what the capture measured from `offsetWidth - clientWidth` minus the borders.
fn pane(scrollbar_gutter: f64, style: &[(&str, &str)]) -> Node {
    Node {
        writing_mode: Default::default(),
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: "/body/div".into(),
        parent: None,
        tag: "div".into(),
        text: String::new(),
        attributes: Attributes::new(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 301.0,
            height: 200.0,
        },
        scrollbar_gutter,
        style: style
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
        before: None,
        after: None,
        ..Default::default()
    }
}

fn compensated(scrollbar_gutter: f64, style: &[(&str, &str)]) -> Option<String> {
    let mut styles = Styles::new();
    preserve_space(&mut styles, &pane(scrollbar_gutter, style), None, &VIEWPORT);
    styles.get("width").cloned()
}

/// The invariant, stated as a relation rather than an example.
///
/// All four panes are identical in the captured evidence - a measured gutter of 10 and a
/// resolved width of 290 - and differ only in the declaration that produced that gutter.
/// They must therefore emit one width, and that width is the 300px the author asked for:
/// the compensation exists to undo the engine's subtraction, not to invent space.
///
/// Written this way it covers the cases no `overflow-y` read can separate. `scroll` reserves
/// a gutter unconditionally where `auto` reserves one only while content overflows;
/// `scrollbar-gutter: stable` reserves one with no scrollbar shown; and a specified
/// `visible` computes to `auto` whenever the other axis scrolls, so the declaration that
/// made the pane scrollable need not be on this axis at all. An assertion keyed to one
/// keyword closes one cell and leaves the rest short by a scrollbar.
#[test]
fn panes_with_one_measured_gutter_emit_one_width() {
    let equivalent: [&[(&str, &str)]; 4] = [
        &[
            ("width", "290px"),
            ("overflow-y", "auto"),
            ("scrollbar-width", "thin"),
        ],
        &[
            ("width", "290px"),
            ("overflow-y", "scroll"),
            ("scrollbar-width", "thin"),
        ],
        &[
            ("width", "290px"),
            ("overflow-y", "auto"),
            ("scrollbar-gutter", "stable"),
            ("scrollbar-width", "thin"),
        ],
        &[
            ("width", "290px"),
            ("overflow-x", "scroll"),
            ("overflow-y", "visible"),
            ("scrollbar-width", "thin"),
        ],
    ];
    for style in equivalent {
        assert_eq!(
            compensated(10.0, style).as_deref(),
            Some("300px"),
            "a pane measuring a 10px gutter was not restored to its authored width: {style:?}"
        );
    }
}

/// The other literal. A default-width scrollbar is subtracted from the resolved width
/// exactly as a thin one is - 15px instead of 10px - so a pane that never mentions
/// `scrollbar-width` is short by more, not by less. Compensating only `thin` treats the
/// *difference* between two scrollbar widths as the loss, when the loss is the whole
/// scrollbar. The arithmetic is the only thing that changes between this test and the one
/// above, which is what makes the keyword provably irrelevant.
#[test]
fn a_default_width_scrollbar_is_compensated_exactly_as_a_thin_one() {
    assert_eq!(
        compensated(15.0, &[("width", "285px"), ("overflow-y", "scroll")]).as_deref(),
        Some("300px")
    );
}

/// The opposite defect, which is the one a hasty widening lands on.
///
/// The first two panes are scroll-adjacent but reserve no space, so the capture measured
/// nothing. The third is the trap that rules out re-deriving the gutter from the recorded
/// geometry: a `<ul>` is 40px wider than its resolved width because the user agent gives it
/// `padding-left: 40px`, and that padding is pruned from `style` precisely because it is the
/// user agent's own. Subtracting the padding that survives pruning reads the whole 40px as a
/// scrollbar. The last pane is a border-box scroller, whose resolved width already spans the
/// scrollbar, so adding the measured gutter would invent space.
#[test]
fn a_pane_that_reserved_nothing_keeps_its_authored_width() {
    let untouched: [(&str, f64, &[(&str, &str)]); 4] = [
        (
            "overflow-y: hidden",
            0.0,
            &[("width", "300px"), ("overflow-y", "hidden")],
        ),
        (
            "overflow: clip",
            0.0,
            &[("width", "300px"), ("overflow-y", "clip")],
        ),
        (
            "user-agent padding pruned from style",
            0.0,
            &[("width", "320px")],
        ),
        (
            "border-box scroller",
            10.0,
            &[
                ("width", "301px"),
                ("box-sizing", "border-box"),
                ("overflow-y", "scroll"),
            ],
        ),
    ];
    for (name, scrollbar_gutter, style) in untouched {
        assert_eq!(
            compensated(scrollbar_gutter, style),
            None,
            "{name} reserved no gutter but was widened anyway"
        );
    }
}
