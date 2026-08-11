//! The membership axis of the stand-in: *which* of the replaced element's attributes may
//! be asserted by the element that replaces it.
//!
//! `source_svg_image_tests` covers the position axis — root versus descendant — which
//! `1c2b132` made exhaustive. Membership was transcribed unexamined: a four-name list
//! carrying `aria-hidden`, the one accessibility attribute whose meaning was already
//! identical to the hardcoded `alt={""}` beside it, and dropping every attribute that
//! names the graphic. Both axes must stay independently pinned, because a fix to either
//! one can silently widen the other.

use super::stand_in::image;

/// The defect. Relocation makes the name unreachable rather than merely moving it: through
/// `<img src>` the graphic becomes an image document, whose internal `role` and `aria-*`
/// are never exposed. The bytes survive in the asset and name nothing, so the stand-in's
/// `alt` is the only channel left and must carry the name across.
#[test]
fn carries_the_graphics_name_onto_the_channel_that_survives_relocation() {
    let img = image(
        concat!(
            r#"<svg role={"img"} aria-label={"Sync conflicts"} viewBox={"0 0 24 24"}>"#,
            r#"<path d={"M4 4h16v16H4z"} /></svg>"#,
        ),
        "a.svg",
    );
    assert!(
        img.contains(r#"alt={"Sync conflicts"}"#),
        "the stand-in cannot be named: {img}"
    );
}

/// An empty `alt` is an assertion, not an omission: it removes the image from the
/// accessibility tree, so a named graphic emitted this way tells the reader it conveys
/// nothing. The wrong artifact and the right one are otherwise byte-identical.
#[test]
fn does_not_declare_a_named_graphic_decorative() {
    let img = image(
        r#"<svg role={"img"} aria-label={"Sync conflicts"}><path d={"M4 4h16v16H4z"} /></svg>"#,
        "a.svg",
    );
    assert!(!img.contains(r#"alt={""}"#), "{img}");
}

/// `aria-label` is translated, not copied. The pair `alt={""} aria-label={"…"}` is valid
/// and the label would win, but ARIA-in-HTML discourages overriding `alt` on an image, and
/// the combination reads as decorative to every linter while announcing text.
#[test]
fn moves_the_name_into_alt_rather_than_leaving_it_on_the_aria_channel() {
    let img = image(r#"<svg aria-label={"Sync conflicts"}></svg>"#, "a.svg");
    assert!(!img.contains("aria-label"), "{img}");
    assert!(img.contains(r#"alt={"Sync conflicts"}"#), "{img}");
}

/// `aria-labelledby` outranks `alt`, and its IDREFs point into the host document where the
/// graphic was inline — so the targets still resolve and it crosses verbatim. Flattening it
/// into `alt` would demote a higher-precedence channel to a lower one, and this stage
/// cannot resolve the reference without inventing text.
#[test]
fn carries_a_higher_precedence_name_reference_on_its_own_channel() {
    let img = image(r#"<svg aria-labelledby={"t1 t2"}></svg>"#, "a.svg");
    assert!(img.contains(r#"aria-labelledby={"t1 t2"}"#), "{img}");
    assert!(img.contains(r#"alt={""}"#), "{img}");
}

/// A description is not a name and belongs on its own channel too. Written as the general
/// case: every `aria-*` the graphic declared describes the content, which the stand-in
/// still shows, so a name the crate has never met crosses with no second code change.
#[test]
fn carries_every_other_aria_attribute_the_graphic_declared() {
    let img = image(
        r#"<svg aria-describedby={"d1"} aria-roledescription={"icon"}></svg>"#,
        "a.svg",
    );
    assert!(img.contains(r#"aria-describedby={"d1"}"#), "{img}");
    assert!(img.contains(r#"aria-roledescription={"icon"}"#), "{img}");
}

/// The inverse guard, and the reason the empty `alt` exists at all. A graphic that named
/// itself nowhere must still be declared decorative, and must gain nothing. Deriving a name
/// from the filename or the markup would turn a silent loss into a confident wrong
/// announcement, which a reviewer reads as plausible and stops checking.
#[test]
fn declares_a_nameless_graphic_decorative() {
    let img = image(
        r#"<svg viewBox={"0 0 24 24"}><path d={"M4 4h16v16H4z"} /></svg>"#,
        "a.svg",
    );
    assert_eq!(img, r#"<img src={"/assets/a.svg"} alt={""} />"#);
}

/// The decorative case stated by the author rather than by omission. `aria-hidden` crosses
/// because it describes the content's exposure, and no name appears to contradict it.
#[test]
fn keeps_an_explicitly_hidden_graphic_hidden_and_unnamed() {
    let img = image(r#"<svg aria-hidden={"true"}></svg>"#, "a.svg");
    assert!(img.contains(r#"aria-hidden={"true"}"#), "{img}");
    assert!(img.contains(r#"alt={""}"#), "{img}");
}

/// `role` describes the element, and that element is gone. An `<img>` already asserts the
/// image role, so copying it is noise, and copying `presentation` or `none` would
/// contradict a non-empty `alt` on the very same tag.
#[test]
fn does_not_copy_the_replaced_elements_role() {
    let named = image(r#"<svg role={"img"} aria-label={"Sync"}></svg>"#, "a.svg");
    assert!(!named.contains("role="), "{named}");
    let presentational = image(r#"<svg role={"presentation"}></svg>"#, "a.svg");
    assert!(!presentational.contains("role="), "{presentational}");
}

/// The over-correction guard. Replacing the list with the crate's usual denylist would
/// carry the source vocabulary onto a host that ignores it: `viewBox`, `fill` and `d` are
/// not global attributes, `<img>` drops them, and they still work where they now live —
/// inside the asset. Emitting them inflates output the project requires to stay readable.
#[test]
fn does_not_carry_the_source_vocabularys_own_attributes() {
    let img = image(
        r##"<svg viewBox={"0 0 24 24"} fill={"#345"} xmlns={"http://www.w3.org/2000/svg"} aria-label={"Sync"}></svg>"##,
        "a.svg",
    );
    for name in ["viewBox", "fill=", "xmlns"] {
        assert!(!img.contains(name), "carried {name}: {img}");
    }
}

/// Membership must not move the position axis. A descendant naming itself names itself, not
/// the graphic, and promoting it would assert the whole icon is that one part.
#[test]
fn does_not_promote_a_descendants_name_onto_the_stand_in() {
    let img = image(
        concat!(
            r#"<svg viewBox={"0 0 24 24"}>"#,
            r#"<path aria-label={"one arrow"} aria-describedby={"d1"} /></svg>"#,
        ),
        "a.svg",
    );
    assert!(!img.contains("aria-label"), "{img}");
    assert!(!img.contains("aria-describedby"), "{img}");
    assert!(img.contains(r#"alt={""}"#), "{img}");
}
