//! Where a replayed phase lands on the page.
//!
//! The layer is portalled to `document.body`, so nothing in its own markup positions it. It
//! used to be pinned to the viewport by a hardcoded `inset:0;width:100vw;height:100vh`, which
//! was right for the one shape that could ever be recorded — a full-page curtain. Now that
//! any phase is recorded, placement has to come from the measurement.

use super::Replay;
use crate::generate::project_test_support::{startup_root as root, startup_state as placed};
use crate::generate::startup_overlays;

/// An inline placeholder standing in for a card belongs on that card, not in the corner of
/// the viewport.
#[test]
fn the_layer_is_placed_on_the_box_the_phase_occupied() {
    let variables = Replay::of(&placed(vec![root(24.0, 120.0, 320.0, 96.0)])).style_variables();

    assert!(
        variables.contains(r#""--recreate-startup-x":"24px""#),
        "{variables}"
    );
    assert!(
        variables.contains(r#""--recreate-startup-y":"120px""#),
        "{variables}"
    );
    assert!(
        variables.contains(r#""--recreate-startup-width":"320px""#),
        "{variables}"
    );
    assert!(
        variables.contains(r#""--recreate-startup-height":"96px""#),
        "{variables}"
    );
}

/// A full-viewport curtain measures the viewport, so the one measured rule reproduces the
/// constant it replaced. This is what proves the placement was generalised rather than
/// swapped for a different special case.
#[test]
fn a_full_viewport_curtain_still_covers_the_viewport() {
    let variables = Replay::of(&placed(vec![root(0.0, 0.0, 1280.0, 720.0)])).style_variables();

    assert!(variables.contains(r#""--recreate-startup-x":"0px""#));
    assert!(variables.contains(r#""--recreate-startup-width":"1280px""#));
    assert!(variables.contains(r#""--recreate-startup-height":"720px""#));
}

/// A phase with several roots is one layer, so the layer must contain all of them. Taking
/// the first root's rect would clip every root after it out of view.
#[test]
fn several_roots_are_contained_by_one_box() {
    let variables = Replay::of(&placed(vec![
        root(10.0, 40.0, 100.0, 50.0),
        root(200.0, 20.0, 60.0, 200.0),
    ]))
    .style_variables();

    assert!(
        variables.contains(r#""--recreate-startup-x":"10px""#),
        "{variables}"
    );
    assert!(
        variables.contains(r#""--recreate-startup-y":"20px""#),
        "{variables}"
    );
    assert!(
        variables.contains(r#""--recreate-startup-width":"250px""#),
        "{variables}"
    );
    assert!(
        variables.contains(r#""--recreate-startup-height":"200px""#),
        "{variables}"
    );
}

/// Descendants are carried inside the layer's own markup, so letting one widen the box would
/// place the layer by a child's geometry instead of the phase's.
#[test]
fn a_descendant_does_not_widen_the_box() {
    let mut child = root(0.0, 0.0, 4000.0, 4000.0);
    child.parent = Some("startup>24".into());

    let variables =
        Replay::of(&placed(vec![root(24.0, 120.0, 320.0, 96.0), child])).style_variables();

    assert!(
        variables.contains(r#""--recreate-startup-width":"320px""#),
        "{variables}"
    );
}

/// The overlay CSS must read the measured placement rather than restate a size. A hardcoded
/// viewport-sized box here would silently override whatever the replay measured.
#[test]
fn the_overlay_css_reads_the_measured_placement() {
    let mut css = String::new();
    startup_overlays::append(&[placed(vec![root(24.0, 120.0, 320.0, 96.0)])], &mut css);

    assert!(css.contains("var(--recreate-startup-x"), "{css}");
    assert!(css.contains("var(--recreate-startup-width"), "{css}");
    assert!(
        !css.contains("100vw!important"),
        "the layer must not be forced to the viewport: {css}"
    );
}
