//! What a viewport band may state, and what the re-emitted condition already covers.

use super::{node, scene};
use crate::generate::authored_conditions::restore_unconditional;
use crate::generate::authored_css_index::Index;
use crate::model::Styles;

/// The band emitters run the same stage over a delta, so a property the band did not change
/// must not be introduced by it with a value the band never measured.
#[test]
fn writes_only_properties_the_condition_actually_declares() {
    let node = node(
        "unsampled",
        &[("background-color", "rgb(0, 0, 255)"), ("height", "40px")],
    );
    let mut styles = Styles::new();
    restore_unconditional(
        &mut styles,
        &node,
        &Index::new(&scene("unsampled", "(min-width: 600px)", "rgb(0, 0, 255)")),
    );

    assert_eq!(styles.len(), 1);
    assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
}

/// The band is not the carrier for a difference the author wrote a condition for. A band is
/// quantised to the widths the capture sampled, so a paint authored at `max-width: 800px` and
/// restated in the 391-768 band is wrong at every width between 769 and 800; the re-emitted
/// condition states it at the author's own breakpoint instead.
#[test]
fn withdraws_from_a_band_what_the_re_emitted_condition_states_at_the_authored_breakpoint() {
    let narrow = node("controltoken", &[("fill", "rgb(200, 10, 10)")]);
    let captured = vec!["@media(max-width:800px){.controltoken{fill:rgb(200, 10, 10);}}".into()];
    let mut delta = narrow.style.clone();
    restore_unconditional(&mut delta, &narrow, &Index::new(&captured));

    assert!(!delta.contains_key("fill"));
}
