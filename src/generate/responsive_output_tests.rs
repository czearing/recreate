use crate::generate::responsive::output_declarations;
use crate::model::Styles;
use std::collections::BTreeMap;

fn render(pairs: &[(&str, &str)]) -> String {
    let styles: Styles = pairs
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect();
    output_declarations(&styles, &BTreeMap::new())
}

/// The defect this file exists for. `list-style` matched none of the 23 prefixes and
/// none of the ~43 exact names the emitter used to filter on - the nearest prefix was
/// `line-`, not `list-`. Because `list-style-type`'s initial value is `disc` and the
/// item still arrives as `display: list-item`, dropping the declaration did not merely
/// lose styling, it painted a bullet the source never had.
#[test]
fn a_list_marker_reset_reaches_the_stylesheet() {
    let css = render(&[("list-style-type", "none"), ("display", "list-item")]);
    assert!(css.contains("list-style-type:none"), "{css}");
    assert!(css.contains("display:list-item"), "{css}");
}

/// The siblings of the same root cause. Each was dropped by the same function for the
/// same non-reason, so each must now survive for the same reason. A fix that hand-listed
/// `list-style` would leave every one of these failing.
#[test]
fn properties_the_old_allow_list_never_named_reach_the_stylesheet() {
    for (name, value) in [
        ("rotate", "45deg"),
        ("scale", "1.5"),
        ("mask-image", "url(a.png)"),
        ("backdrop-filter", "blur(4px)"),
        ("direction", "rtl"),
        ("writing-mode", "vertical-rl"),
        ("list-style-position", "inside"),
        ("list-style-image", "url(dot.png)"),
        ("caret-shape", "block"),
    ] {
        let css = render(&[(name, value)]);
        assert!(css.contains(&format!("{name}:{value}")), "{name}: {css}");
    }
}

/// The emitter must not acquire a new criterion of its own. Whether a declaration is
/// worth emitting is settled upstream by measurement against the element's
/// no-author-CSS baseline, so anything that survives that far is load-bearing and the
/// emitter's only remaining job is to drop declarations made inert by another one.
#[test]
fn an_unrecognised_property_is_not_second_guessed() {
    let css = render(&[("anchor-name", "--pop"), ("field-sizing", "content")]);
    assert!(css.contains("anchor-name:--pop"), "{css}");
    assert!(css.contains("field-sizing:content"), "{css}");
}

/// Kept from the deleted normalize step: a side with no border draws nothing, so its
/// width and colour describe an edge that does not exist. This is a statement about a
/// pair of values, not about a property name, which is why it survives.
///
/// The style keyword is the determinant of that pair, not a third member of it. It is
/// what makes the other two inert, and it is the only one of the three that can beat the
/// user agent's own border on the tag the class lands on. This assertion used to read
/// `!css.contains("border-top")`, which pinned its deletion as intended behaviour.
#[test]
fn a_border_side_that_draws_nothing_keeps_its_style_and_drops_its_width_and_colour() {
    let css = render(&[
        ("border-top-style", "none"),
        ("border-top-width", "3px"),
        ("border-top-color", "rgb(1, 2, 3)"),
        ("border-left-style", "solid"),
        ("border-left-width", "2px"),
    ]);
    assert!(css.contains("border-top-style:none"), "{css}");
    assert!(!css.contains("border-top-width"), "{css}");
    assert!(!css.contains("border-top-color"), "{css}");
    assert!(css.contains("border-left-width:2px"), "{css}");
}

/// An inset only positions a positioned box, so `auto` on a static one is sampling
/// noise. On a positioned box the same `auto` is load-bearing - it is what stops an
/// authored offset from applying on the opposite edge.
#[test]
fn an_auto_inset_is_noise_on_a_static_box_and_evidence_on_a_positioned_one() {
    let statik = render(&[("position", "static"), ("top", "auto"), ("left", "auto")]);
    assert!(!statik.contains("top:auto"), "{statik}");
    let positioned = render(&[("position", "absolute"), ("top", "auto"), ("left", "10px")]);
    assert!(positioned.contains("top:auto"), "{positioned}");
}

/// A shorthand only reaches a style map from the authored index; the capture enumerates
/// longhands. Sorted emission puts `padding` before `padding-top`, so once the longhands
/// are present the shorthand is overridden on the next line and is pure duplication.
#[test]
fn a_shorthand_its_own_longhands_override_is_not_repeated() {
    let css = render(&[
        ("padding", "40px"),
        ("padding-top", "40px"),
        ("padding-right", "40px"),
        ("padding-bottom", "40px"),
        ("padding-left", "40px"),
    ]);
    assert!(!css.contains("padding:40px"), "{css}");
    assert!(css.contains("padding-top:40px"), "{css}");
}

/// The removal is about being overridden, not about being a shorthand. A shorthand that
/// carries the only copy of the value must survive, or the declaration is lost outright.
#[test]
fn a_shorthand_no_longhand_replaces_is_kept() {
    let css = render(&[("padding", "2vw")]);
    assert!(css.contains("padding:2vw"), "{css}");
}

/// `gap` and `inset` are among the families whose parts are not spelled as their prefix.
/// A partial set of parts leaves the shorthand carrying the rest, so it stays.
#[test]
fn a_shorthand_only_partly_replaced_is_kept() {
    let css = render(&[("gap", "8px 4px"), ("row-gap", "8px")]);
    assert!(css.contains("gap:8px 4px"), "{css}");
    let full = render(&[("gap", "8px"), ("row-gap", "8px"), ("column-gap", "8px")]);
    let replaced = full
        .replace("row-gap:8px;", "")
        .replace("column-gap:8px;", "");
    assert!(!replaced.contains("gap:"), "{replaced}");
}

/// `flex-flow` is renamed exactly as `gap` and `inset` are, and sorted emission places it
/// between the two longhands that spell it out — so it overrode `flex-direction` with its
/// own copy while claiming to be inert. Reading the renames from their one owner is what
/// reaches it; a family listed twice was only ever going to be listed short in one place.
#[test]
fn a_renamed_shorthand_its_longhands_override_is_not_repeated() {
    let css = render(&[
        ("flex-flow", "row wrap"),
        ("flex-direction", "row"),
        ("flex-wrap", "wrap"),
    ]);
    assert!(!css.contains("flex-flow:"), "{css}");
    assert!(css.contains("flex-direction:row"), "{css}");

    let partial = render(&[("flex-flow", "row wrap"), ("flex-direction", "row")]);
    assert!(partial.contains("flex-flow:row wrap"), "{partial}");
}

/// A longhand that merely shares a prefix with another longhand is not a shorthand of
/// it. `color-scheme` must never delete `color`, which would drop every authored colour.
#[test]
fn a_longhand_sharing_a_prefix_is_not_treated_as_a_shorthand() {
    let css = render(&[("color", "rgb(17, 17, 17)"), ("color-scheme", "dark")]);
    assert!(css.contains("color:rgb(17, 17, 17)"), "{css}");
}

/// An origin's computed value is always the used pixel pair, because percentages resolve
/// against the border box — so on this path, where nothing above replaced the capture
/// with an authored spelling, every origin is a measurement of a box the recreation
/// already reproduces. Emitting it anchors correctly at exactly one viewport.
///
/// This used to be decided by whether anything was transformed, which asked the wrong
/// question in two directions at once: it emitted the measurement whenever something was
/// (below), and it deleted an authored anchor whose transform arrives in another state
/// (see `responsive_anchor_tests`).
#[test]
fn a_sampled_origin_is_not_emitted() {
    let css = render(&[
        ("transform-origin", "295.281px 20px"),
        ("perspective-origin", "295.281px 20px"),
        ("width", "600px"),
    ]);
    assert!(!css.contains("origin"), "{css}");
}

/// And no route to a transform rescues it. Each of these once did.
#[test]
fn no_anchor_makes_a_sampled_origin_emittable() {
    for anchor in [
        ("transform", "rotate(3deg)"),
        ("rotate", "3deg"),
        ("scale", "1.5"),
        ("translate", "10px"),
        ("animation-name", "spin"),
    ] {
        let css = render(&[("transform-origin", "0px 0px"), anchor]);
        assert!(!css.contains("transform-origin"), "{} {css}", anchor.0);
    }
    let css = render(&[("perspective-origin", "0px 0px"), ("perspective", "800px")]);
    assert!(!css.contains("perspective-origin"), "{css}");
}

/// A band with nothing in it is four empty `@media` blocks at the foot of the sheet.
#[test]
fn an_empty_band_emits_no_media_block() {
    use crate::generate::responsive::rules::media_rule;
    assert_eq!(media_rule(Some(769), 1440, ""), "");
    assert_eq!(media_rule(None, 320, "  \n"), "");
    assert!(media_rule(None, 320, ".r0{color:red}").contains("@media"));
}
