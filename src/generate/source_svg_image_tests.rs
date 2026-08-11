use super::stand_in::image;

/// The observable form of the defect. An inline SVG is replaced by an `<img>` that stands
/// in for it, and the stand-in may only declare what the graphic itself declared. The root
/// here carries `height` and no `width`, which is the ordinary shape of an icon sized by a
/// stylesheet; the `<rect>` is the only other holder of either name.
///
/// Absence is the correct output, not a default. `width` and `height` map to the CSS
/// dimension properties as presentational hints and, when both parse, to
/// `aspect-ratio: auto w / h` — so one harvested dimension beside one real one asserts a
/// ratio the graphic never had, and the reserved box collapses when the asset loads.
#[test]
fn sizes_the_stand_in_only_by_what_the_replaced_element_declared() {
    let img = image(
        concat!(
            r#"<svg height={"120"} viewBox={"0 0 240 120"}>"#,
            r#"<rect width={"20"} height={"40"} /></svg>"#,
        ),
        "a.svg",
    );
    assert!(img.contains(r#"height={"120"}"#), "{img}");
    assert!(
        !img.contains("width="),
        "gave the stand-in a width its subject never declared: {img}"
    );
}

/// The same rule for the two attributes that carry meaning rather than size. A decorative
/// child's class would decide which rules style the whole graphic, and its hidden flag
/// would take the graphic out of the accessibility tree entirely.
#[test]
fn does_not_promote_a_decorative_child_class_or_hidden_flag_onto_the_stand_in() {
    let img = image(
        concat!(
            r#"<svg viewBox={"0 0 24 24"}>"#,
            r#"<path className={"r_decor"} aria-hidden={"true"} /></svg>"#,
        ),
        "a.svg",
    );
    assert!(!img.contains("className"), "{img}");
    assert!(!img.contains("aria-hidden"), "{img}");
    assert!(img.contains(r#"src={"/assets/a.svg"}"#), "{img}");
}

/// What the root does declare still arrives, so the fix is a narrowing of scope rather
/// than a suppression. Asserted alongside the losses above because a reader that returned
/// nothing at all would satisfy every negative assertion here.
#[test]
fn keeps_every_attribute_the_replaced_element_declared_itself() {
    let img = image(
        concat!(
            r#"<svg className={"r_root"} aria-hidden={"true"} height={"120"} width={"240"}>"#,
            r#"<rect width={"20"} /></svg>"#,
        ),
        "a.svg",
    );
    assert!(img.contains(r#"className={"r_root"}"#), "{img}");
    assert!(img.contains(r#"aria-hidden={"true"}"#), "{img}");
    assert!(img.contains(r#"height={"120"}"#), "{img}");
    assert!(img.contains(r#"width={"240"}"#), "{img}");
}
