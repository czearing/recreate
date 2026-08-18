//! Reading back the division the engine performed, rather than transcribing its grammar.
//!
//! The share each longhand of a shorthand receives is decided by a per-family grammar. Every
//! spelling of that grammar inside this repository would be a table: right about the families
//! somebody listed and silently wrong about the rest. The engine that parsed the sheet has
//! already divided the block, so the capture records what it stored and these assert that the
//! record is what decides — including where it says the division cannot be settled at all.

use super::super::{Claim, claim};
use super::{divided, share};
/// The division the engine performed, read back. Nothing here knows that `padding` gives its
/// first component to the block edges and its second to the inline ones, and nothing needs to.
#[test]
fn reads_the_division_the_engine_recorded_for_the_block() {
    let shorthands = divided(
        "padding: 24px 8px;",
        &[
            ("padding-bottom", "24px"),
            ("padding-left", "8px"),
            ("padding-right", "8px"),
            ("padding-top", "24px"),
        ],
    );
    let block = " padding: 24px 8px; }";

    assert_eq!(
        share(&shorthands, block, "padding", "24px 8px", "padding-top"),
        Some("24px")
    );
    assert_eq!(
        share(&shorthands, block, "padding", "24px 8px", "padding-left"),
        Some("8px")
    );
}

/// A longhand the block never stored is not claimed, whatever the names suggest. This is what
/// keeps `border` off `border-radius` once a division exists to consult.
#[test]
fn refuses_a_longhand_the_recorded_block_never_stored() {
    let shorthands = divided(
        "border: 8px solid rgb(1, 2, 3);",
        &[
            ("border-top-color", "rgb(1, 2, 3)"),
            ("border-top-style", "solid"),
            ("border-top-width", "8px"),
        ],
    );
    let block = "border: 8px solid rgb(1, 2, 3);";

    assert_eq!(
        share(
            &shorthands,
            block,
            "border",
            "8px solid rgb(1, 2, 3)",
            "border-top-width"
        ),
        Some("8px")
    );
    assert!(
        share(
            &shorthands,
            block,
            "border",
            "8px solid rgb(1, 2, 3)",
            "border-radius"
        )
        .is_none(),
        "a corner the block never stored was claimed from the name alone"
    );
}

/// An empty share is the engine's own "declared, and I cannot yet say to what" — a value
/// holding `var()`. It is not absence, and collapsing the two is what would delete a
/// declaration the author wrote.
#[test]
fn keeps_an_unsettled_share_distinct_from_no_share_at_all() {
    let shorthands = divided("padding: 0px var(--gutter);", &[("padding-top", "")]);

    assert!(matches!(
        claim(
            &shorthands,
            "padding: 0px var(--gutter);",
            "padding",
            "0px var(--gutter)",
            "padding-top"
        ),
        Claim::Unsettled
    ));
}
