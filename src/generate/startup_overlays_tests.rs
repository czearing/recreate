use super::append;
use crate::generate::interactions::REDUCED_MOTION_CSS;
use crate::generate::project_test_support::{startup_root, startup_state};

fn recorded() -> crate::model::PageState {
    startup_state(vec![startup_root(0.0, 0.0, 100.0, 100.0)])
}

/// The layer is driven entirely by custom properties, so the emitter's only job is to make the
/// rules that read them. A stylesheet without them replays no phase at all.
#[test]
fn the_layer_reads_the_captured_spans() {
    let mut css = String::new();
    append(std::slice::from_ref(&recorded()), &mut css);

    assert!(css.contains("var(--recreate-startup-duration"), "{css}");
    assert!(css.contains("var(--recreate-startup-delay"), "{css}");
    assert!(
        super::runtime("const View=baselineViews[0];const activate=".into(), &[])
            .contains("startupDone"),
        "the runtime must be able to end the phase"
    );
}

/// A page that recorded no phase must gain none of this. Without it the emitter could satisfy
/// every other assertion by writing the layer unconditionally.
#[test]
fn a_page_without_a_phase_gains_no_rules() {
    let mut css = String::new();
    append(
        &[crate::generate::project_test_support::state(1280)],
        &mut css,
    );

    assert!(css.is_empty(), "{css}");
}

/// The reduced-motion case is already covered twice over, so the emitter must not state it a
/// third time. `interactions::REDUCED_MOTION_CSS` stops every animation on the page, and the
/// layer's own base rule is `opacity:0;visibility:hidden;pointer-events:none` — an unplayed
/// animation therefore leaves it hidden. Restating it emitted a second
/// `@media(prefers-reduced-motion:reduce)` block into the same stylesheet, which is the
/// duplicate a reader has to reconcile before trusting either copy.
#[test]
fn the_reduced_motion_case_is_not_restated() {
    let mut css = String::from(REDUCED_MOTION_CSS);
    let before = css.matches("@media(prefers-reduced-motion:reduce)").count();
    append(std::slice::from_ref(&recorded()), &mut css);

    assert_eq!(
        css.matches("@media(prefers-reduced-motion:reduce)").count(),
        before,
        "the startup layer opened a second reduced-motion block: {css}"
    );
    let hidden = css
        .split_once(".recreateStartupOverlay{")
        .expect("the layer must have a base rule")
        .1;
    assert!(
        hidden.starts_with("opacity:0;visibility:hidden;pointer-events:none"),
        "the layer must be hidden before its animation runs, which is what makes the \
         reduced-motion rule unnecessary: {hidden}"
    );
}
