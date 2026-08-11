use super::stand_in::image;

/// The twins. Both roots size the same graphic to the same 100px box on the source page —
/// the subject through `2em` against a 50px context, the control through `100` — so the
/// stand-ins must agree on how the dimension is treated *and* land on the same size. SVG's
/// `width` takes a CSS `<length>`, so `2em` is valid where it was read; HTML's dimension
/// attributes admit only a valid non-negative integer and reject nothing, error-recovering
/// by keeping the leading digit run. Copied verbatim, `2em` paints a 2x2 image while greping
/// byte-identical to what was captured. Dropping it is not enough either: the asset keeps
/// `width="2em"`, but inside an image document that `em` resolves against the file's own
/// font size, not the 50px context the author wrote it against.
#[test]
fn does_not_recoin_a_css_length_as_a_pixel_count() {
    let arm = |size: &str| {
        image(
            &format!(
                r#"<svg className={{"r_g"}} width={{"{size}"}} height={{"{size}"}} viewBox={{"0 0 10 10"}}><path d={{"M1 1 L9 9"}} /></svg>"#
            ),
            "a.svg",
        )
    };
    let subject = arm("2em");
    assert!(
        !subject.contains("width=") && !subject.contains("height="),
        "recoined a font-relative length as a pixel count: {subject}"
    );
    assert!(
        subject.contains(r#"style={{width:"2em",height:"2em"}}"#),
        "lost the length instead of moving it to the grammar that holds it: {subject}"
    );
    assert!(subject.contains(r#"className={"r_g"}"#), "{subject}");
    let control = arm("100");
    assert!(control.contains(r#"width={"100"}"#), "{control}");
    assert!(!control.contains("style="), "{control}");
}

/// The rule the twins are one case of, stated over the receiving grammar rather than over a
/// list of spellings. `0` is carried because HTML's production admits it; every other value
/// here reaches an `<img>` unremarked by React and is silently truncated by the parser, so
/// each moves to CSS instead, which discards what it cannot parse rather than painting a
/// prefix of it. An absent length is absent in both grammars and asserts nothing anywhere.
#[test]
fn admits_only_a_dimension_html_can_hold_whole() {
    for value in ["100", "0", "0240"] {
        let img = image(&root(value), "a.svg");
        assert!(img.contains(&format!(r#"width={{"{value}"}}"#)), "{img}");
        assert!(!img.contains("style="), "{img}");
    }
    for value in ["2em", "50%", "0.5", "1e3", "-4", " 12", "12px", "auto"] {
        let img = image(&root(value), "a.svg");
        assert!(
            !img.contains("width={"),
            "carried a dimension HTML cannot hold whole ({value:?}): {img}"
        );
        assert!(
            img.contains(&format!(r#"style={{{{width:"{value}"}}}}"#)),
            "dropped a length CSS could have held ({value:?}): {img}"
        );
    }
    let empty = image(&root(""), "a.svg");
    assert!(
        !empty.contains("width") && !empty.contains("style="),
        "{empty}"
    );
}

fn root(width: &str) -> String {
    format!(r#"<svg width={{"{width}"}} viewBox={{"0 0 10 10"}}><path d={{"M1 1"}} /></svg>"#)
}

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
