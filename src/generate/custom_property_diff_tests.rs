use super::against;

#[test]
fn emits_only_responsive_declaration_changes() {
    let base = ":root{--brand:red;}\nmain{color:red;}\nmain{z-index:1;}\n";
    let responsive = ":root{--brand:blue;}\nmain{color:red;}\nmain{z-index:1;}\n";

    assert_eq!(against(base, responsive), ":root{--brand:blue;}\n");
}

/// The base is emitted outside every guard, so a property the base declares and
/// the band does not keeps applying below the breakpoint. Reproducing the
/// absence means writing a cancellation, not omitting a declaration.
#[test]
fn cancels_a_property_the_band_stops_declaring() {
    let base = ":root{--changed:24px;--dropped:37px;}\n";
    let responsive = ":root{--changed:12px;}\n";

    assert_eq!(
        against(base, responsive),
        ":root{--changed:12px;--dropped:initial;}\n"
    );
}

/// The dropped property lives under a selector that still exists on the band
/// side, so no walk keyed by selector can reach it.
#[test]
fn cancels_a_property_whose_selector_survives() {
    let base = ".card{--gap:8px;--accent:red;}\n";
    let responsive = ".card{--gap:8px;}\n";

    assert_eq!(against(base, responsive), ".card{--accent:initial;}\n");
}

/// Scoped per-element rules flow through the same call, so a rule the band drops
/// entirely must cancel every property it carried.
#[test]
fn cancels_every_property_of_a_rule_the_band_drops_entirely() {
    let base = ":root{--brand:red;}\n.panel{--pad:4px;--edge:2px;}\n";
    let responsive = ":root{--brand:red;}\n";

    assert_eq!(
        against(base, responsive),
        ".panel{--edge:initial;--pad:initial;}\n"
    );
}

#[test]
fn emits_a_property_the_band_introduces() {
    let base = ":root{--brand:red;}\n";
    let responsive = ":root{--brand:red;--added:9px;}\n";

    assert_eq!(against(base, responsive), ":root{--added:9px;}\n");
}

/// Inverse guard: the union walk must not start restating what did not change.
/// A property present in both states with the same value belongs to the base
/// alone, and repeating it in the band is over-emission, not fidelity.
#[test]
fn says_nothing_when_both_states_agree() {
    let base = ":root{--brand:red;--gap:8px;}\n.panel{--pad:4px;}\n";

    assert_eq!(against(base, base), "");
}

/// Inverse guard: an unchanged neighbour must not ride along with a changed one.
#[test]
fn leaves_an_unchanged_neighbour_out_of_a_changed_rule() {
    let base = ":root{--brand:red;--gap:8px;}\n";
    let responsive = ":root{--brand:blue;--gap:8px;}\n";

    assert_eq!(against(base, responsive), ":root{--brand:blue;}\n");
}

/// Inverse guard: cancelling is reserved for properties that actually left.
#[test]
fn never_cancels_a_property_the_band_still_declares() {
    let base = ":root{--kept:5px;--gone:7px;}\n";
    let responsive = ":root{--kept:6px;}\n";
    let emitted = against(base, responsive);

    assert!(emitted.contains("--gone:initial;"), "{emitted}");
    assert!(!emitted.contains("--kept:initial;"), "{emitted}");
    assert!(emitted.contains("--kept:6px;"), "{emitted}");
}

/// The producer writes one selector per line and may repeat a selector across
/// lines, so the reader must merge them before either side is compared.
#[test]
fn merges_declarations_a_producer_split_across_lines() {
    let base = ":root{--a:1px;}\n:root{--b:2px;}\n";
    let responsive = ":root{--a:1px;}\n";

    assert_eq!(against(base, responsive), ":root{--b:initial;}\n");
}
