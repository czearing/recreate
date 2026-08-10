//! The emitter's contract is that it never invents a size: an authored size is emitted
//! verbatim, and anything else is left to the flow that sized it in the source. These
//! tests pin both halves of that contract — what may not be written, and what a later
//! stage may not take back.

use super::*;
use crate::model::{Node, Rect, Styles, Viewport};
use std::collections::BTreeMap;

fn viewport() -> Viewport {
    Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    }
}

/// Splits emitted `name:value;` text so a property can be asserted by name. Testing
/// `contains("height:100vh")` would match `min-height:100vh`, which is the very
/// distinction under test.
fn declared(css: &str) -> BTreeMap<&str, &str> {
    css.split(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .map(|(name, value)| (name.trim(), value.trim()))
        .collect()
}

fn element(class: &str, y: f64, height: f64) -> Node {
    Node {
        disabled: false,
        path: "html>body>section".into(),
        parent: Some("html>body".into()),
        tag: "section".into(),
        text: String::new(),
        attributes: crate::model::Attributes::from([("class".into(), class.into())]),
        rect: Rect {
            x: 120.0,
            y,
            width: 1200.0,
            height,
        },
        style: Styles::from([
            ("box-sizing".into(), "border-box".into()),
            ("width".into(), "1200px".into()),
            ("height".into(), format!("{height}px")),
            ("min-height".into(), format!("{height}px")),
        ]),
        before: None,
        after: None,
    }
}

fn emit(node: &Node, rules: &[String]) -> String {
    base_declarations(node, None, &viewport(), &Default::default(), rules, false)
}

/// The author asked for a floor, not a size. A box that reaches the viewport bottom
/// because `min-height` put it there has no authored height, and writing one clamps the
/// box the source deliberately let grow. Measured geometry cannot tell an authored
/// `height: 100vh` from a `min-height`, a stretched flex item or a percentage chain —
/// they all produce the same box — so geometry may not be used to decide it.
#[test]
fn never_invents_a_height_for_a_box_whose_author_wrote_only_min_height() {
    let subject = element("panel", 0.0, 900.0);
    let css = emit(&subject, &[".panel { min-height: 100vh; }".into()]);
    let declared = declared(&css);
    assert_eq!(declared.get("min-height"), Some(&"100vh"));
    assert_eq!(declared.get("height"), None, "invented a height: {css}");
}

/// The control twin: identical authored CSS, pushed clear of the viewport bottom. It
/// separates "no height is invented" from "the stage never ran".
#[test]
fn never_invents_a_height_for_a_box_clear_of_the_viewport_bottom() {
    let control = element("panel", 940.0, 900.0);
    let css = emit(&control, &[".panel { min-height: 100vh; }".into()]);
    let declared = declared(&css);
    assert_eq!(declared.get("min-height"), Some(&"100vh"));
    assert_eq!(declared.get("height"), None);
}

/// Both twins author the identical declaration, so the emitted sizing must be identical.
/// Any difference between them is decided by geometry, which is exactly the defect.
#[test]
fn emits_the_same_sizing_for_twins_that_author_the_same_declaration() {
    let rules = [".panel { min-height: 100vh; }".to_string()];
    let subject_css = emit(&element("panel", 0.0, 900.0), &rules);
    let control_css = emit(&element("panel", 940.0, 900.0), &rules);
    let heights = |css: &str| {
        declared(css)
            .into_iter()
            .filter(|(name, _)| name.contains("height"))
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect::<Vec<_>>()
    };
    assert_eq!(heights(&subject_css), heights(&control_css));
}

/// The heuristic was right about this case, and deleting it must not lose it. An
/// authored full-viewport height is restored by the authored index, not inferred.
#[test]
fn keeps_a_height_the_author_actually_wrote() {
    let subject = element("panel", 0.0, 900.0);
    let css = emit(&subject, &[".panel { height: 100vh; }".into()]);
    assert_eq!(declared(&css).get("height"), Some(&"100vh"));
}

/// The guard's own job. A pixel nobody authored is the sampled used value, and emitting
/// it pins the box to the captured viewport.
#[test]
fn drops_a_sampled_pixel_size_the_author_never_wrote() {
    let subject = element("panel", 940.0, 480.0);
    let css = emit(&subject, &[]);
    let declared = declared(&css);
    assert_eq!(declared.get("height"), None);
    assert_eq!(declared.get("width"), None);
}

/// The guard must still reconcile a genuine sample against the authored text, including
/// values the authored index itself declines to restore because it cannot compare them
/// to the sample. Losing this re-freezes every fluid length at the captured viewport.
#[test]
fn restores_an_authored_fluid_length_over_the_sampled_pixel() {
    let mut subject = element("panel", 940.0, 480.0);
    subject.style.insert("width".into(), "1200px".into());
    let css = emit(
        &subject,
        &[".panel { width: clamp(20rem, 60vw, 75rem); }".into()],
    );
    assert_eq!(
        declared(&css).get("width"),
        Some(&"clamp(20rem, 60vw, 75rem)")
    );
}

/// The opposite failure, in the one stage that writes a size in pixels on purpose.
/// `preserve_space` widens a thin-scrollbar container by its gutter, because a thin
/// scrollbar is 10px where the default is 15px and dropping the difference makes every
/// such container narrower than the source. That value is emitter output whose `px` tail
/// is arithmetic, not provenance, and the guard deletes it.
#[test]
fn keeps_the_scrollbar_gutter_a_geometry_stage_reserved() {
    let pane = Node {
        tag: "div".into(),
        rect: Rect {
            x: 100.0,
            y: 0.0,
            width: 310.0,
            height: 400.0,
        },
        style: Styles::from([
            ("scrollbar-width".into(), "thin".into()),
            ("overflow-y".into(), "auto".into()),
            ("width".into(), "300px".into()),
        ]),
        ..element("pane", 0.0, 400.0)
    };
    let css = emit(&pane, &[]);
    assert_eq!(declared(&css).get("width"), Some(&"310px"));
}

/// The opposite failure. A width the geometry stage computed on purpose is emitter
/// output, not a sample, and its `px` tail is an accident of arithmetic. Deleting it
/// reinstates the overflow the calc exists to prevent.
#[test]
fn keeps_a_size_a_geometry_stage_computed() {
    let bar = Node {
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 200.0,
        },
        style: Styles::from([
            ("box-sizing".into(), "content-box".into()),
            ("width".into(), "1376px".into()),
            ("padding".into(), "32px".into()),
        ]),
        ..element("bar", 0.0, 200.0)
    };
    let css = emit(&bar, &[]);
    assert_eq!(declared(&css).get("width"), Some(&"calc(100% - 64px)"));
}

/// The same rule for the fill the width stage states explicitly. It is the commonest
/// value that stage writes, and no lexical test can tell it from an authored one.
#[test]
fn keeps_an_explicit_fill_a_geometry_stage_computed() {
    let bar = Node {
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 200.0,
        },
        style: Styles::from([
            ("box-sizing".into(), "border-box".into()),
            ("width".into(), "1440px".into()),
        ]),
        ..element("bar", 0.0, 200.0)
    };
    let css = emit(&bar, &[]);
    assert_eq!(declared(&css).get("width"), Some(&"100%"));
}
