use super::*;

/// A size the source never authored cannot be recovered from a sample: the capture records
/// the pixels the box happened to occupy at one viewport. So an unauthored size is dropped
/// and the box is sized by the same flow that sized it in the source.
#[test]
fn drops_a_control_width_the_source_never_authored() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let control = node("button", 0.0, 28.0);
    let parent = node("div", 0.0, 28.0);
    let css = base_declarations(
        &control,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
    );
    assert!(!css.contains("width:28px"), "{css}");
}

/// A size the source did author is emitted verbatim, whatever the element is. This is the
/// only way a fixed size survives, and it needs no per-tag rule to recognise it.
#[test]
fn keeps_a_control_width_the_source_authored() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let control = node("button", 0.0, 28.0);
    let parent = node("div", 0.0, 28.0);
    let css = base_declarations(
        &control,
        Some(&parent),
        &viewport,
        &Default::default(),
        &["*{width:28px;}".to_string()],
        false,
    );
    assert!(css.contains("width:28px"), "{css}");
}

/// Authored CSS declares what differs from the initial value. Emitting every computed
/// property instead buries the handful of real declarations under browser defaults.
/// Margin and padding are excluded: the browser's own stylesheet sets those, so a `0px`
/// there is a reset the recreation still needs.
#[test]
fn drops_declarations_that_only_restate_a_browser_default() {
    let styles = Styles::from([
        ("display".into(), "flex".into()),
        ("align-items".into(), "normal".into()),
        ("flex-basis".into(), "auto".into()),
        ("flex-direction".into(), "row".into()),
        ("flex-grow".into(), "0".into()),
        ("flex-shrink".into(), "1".into()),
        ("flex-wrap".into(), "nowrap".into()),
        ("justify-content".into(), "normal".into()),
        ("left".into(), "auto".into()),
        ("margin-top".into(), "0px".into()),
        ("padding-top".into(), "0px".into()),
    ]);
    let css = crate::generate::responsive::output_declarations(&styles, &Default::default());
    assert_eq!(css, "display:flex;margin-top:0px;padding-top:0px;", "{css}");
}

/// An inset is load-bearing on a positioned box: it is what stops an authored offset on
/// the opposite edge from applying.
#[test]
fn keeps_an_auto_inset_on_a_positioned_box() {
    let styles = Styles::from([
        ("position".into(), "absolute".into()),
        ("left".into(), "auto".into()),
        ("right".into(), "20px".into()),
    ]);
    let css = crate::generate::responsive::output_declarations(&styles, &Default::default());
    assert!(css.contains("left:auto"), "{css}");
}

/// A card's background overlay is pinned to all four edges of the card, so the card sizes
/// it. Emitting the pixels it measured at the widest viewport leaves the overlay hanging
/// outside the card on every narrower screen.
#[test]
fn drops_the_sampled_box_of_an_overlay_pinned_to_every_edge() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut overlay = node("div", 0.0, 313.328);
    overlay.style.extend([
        ("position".into(), "absolute".into()),
        ("top".into(), "0px".into()),
        ("right".into(), "0px".into()),
        ("bottom".into(), "0px".into()),
        ("left".into(), "0px".into()),
        ("width".into(), "313.328px".into()),
        ("height".into(), "168px".into()),
    ]);
    let parent = node("div", 0.0, 313.328);
    let css = base_declarations(
        &overlay,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
    );
    assert!(!css.contains("width:313.328px"), "{css}");
    assert!(!css.contains("height:168px"), "{css}");
}

/// An overlay anchored to only one edge still needs its measured size to exist at all.
#[test]
fn keeps_the_sampled_box_of_an_overlay_anchored_to_one_edge() {
    let styles = Styles::from([
        ("position".into(), "absolute".into()),
        ("top".into(), "0px".into()),
        ("left".into(), "0px".into()),
        ("right".into(), "auto".into()),
        ("bottom".into(), "auto".into()),
        ("width".into(), "313.328px".into()),
        ("height".into(), "168px".into()),
    ]);
    let css = crate::generate::responsive::output_declarations(&styles, &Default::default());
    assert!(css.contains("width:313.328px"), "{css}");
    assert!(css.contains("height:168px"), "{css}");
}

/// A card body is sized by the rows inside it. Emitting the height sampled at one viewport
/// freezes it, so when its content rewraps at a narrower width the extra rows spill out of
/// the card and every section below it shifts up.
#[test]
fn drops_the_sampled_height_of_a_box_that_holds_flow_content() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut card = node("div", 0.0, 313.0);
    card.style.insert("display".into(), "block".into());
    card.style.insert("height".into(), "168px".into());
    let parent = node("div", 0.0, 313.0);
    let css = crate::generate::responsive::base_declarations_indexed(
        &card,
        Some(&parent),
        &viewport,
        &Default::default(),
        &crate::generate::authored_css::Index::new(&[]),
        false,
    );
    assert!(!css.contains("height:168px"), "{css}");
}

/// An empty box has no content to size it, but a height it never authored is still only a
/// sample. The source collapsed it too unless something authored the height, and the
/// authored height is what carries it.
#[test]
fn drops_the_sampled_height_of_a_box_with_no_flow_content() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut spacer = node("div", 0.0, 313.0);
    spacer.style.insert("display".into(), "block".into());
    spacer.style.insert("height".into(), "168px".into());
    let parent = node("div", 0.0, 313.0);
    let css = crate::generate::responsive::base_declarations_indexed(
        &spacer,
        Some(&parent),
        &viewport,
        &Default::default(),
        &crate::generate::authored_css::Index::new(&[]),
        false,
    );
    assert!(!css.contains("height:168px"), "{css}");
}

/// A headline is sized by its text. Emitting the width and height sampled while it fitted
/// on one line stops it rewrapping, so at any narrower viewport it runs off the right edge
/// instead of flowing onto a second line.
#[test]
fn drops_the_sampled_box_of_a_heading_that_has_to_rewrap() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut heading = node("h1", 0.0, 558.188);
    heading.rect.height = 41.3958;
    heading.style.insert("width".into(), "558.188px".into());
    heading.style.insert("height".into(), "41.3958px".into());
    heading.style.insert("display".into(), "block".into());
    heading.style.insert("position".into(), "static".into());
    let mut parent = node("div", 0.0, 980.0);
    parent.style.insert("display".into(), "flex".into());
    parent
        .style
        .insert("flex-direction".into(), "column".into());
    parent
        .style
        .insert("align-items".into(), "flex-start".into());
    let css = base_declarations(
        &heading,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
    );
    assert!(!css.contains("width:558"), "{css}");
    assert!(!css.contains("height:41"), "{css}");
}

/// A clipping box relies on its height to do the clipping. That height is load-bearing
/// because the source authored it, and the authored value is what survives — the emitter
/// does not need to know that `overflow:hidden` makes a height special.
#[test]
fn keeps_the_authored_height_of_a_clipping_text_box() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut summary = node("p", 0.0, 400.0);
    summary.rect.height = 40.0;
    summary.style.insert("width".into(), "400px".into());
    summary.style.insert("height".into(), "40px".into());
    summary.style.insert("display".into(), "block".into());
    summary.style.insert("position".into(), "static".into());
    summary.style.insert("overflow".into(), "hidden".into());
    let mut parent = node("div", 0.0, 980.0);
    parent.style.insert("display".into(), "flex".into());
    parent
        .style
        .insert("flex-direction".into(), "column".into());
    let css = base_declarations(
        &summary,
        Some(&parent),
        &viewport,
        &Default::default(),
        &["*{height:40px;}".to_string()],
        false,
    );
    assert!(css.contains("height:40px"), "{css}");
}

/// The sampled width of a labelled button is its label's width at capture. Emitting it
/// leaves no slack, so text metrics that resolve a fraction of a pixel wider wrap the label
/// onto a second line and make the button twice as tall.
#[test]
fn drops_the_sampled_width_of_a_labelled_button() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut control = node("button", 0.0, 136.021);
    control.rect.height = 36.0;
    control.style.insert("width".into(), "136.021px".into());
    let parent = node("div", 0.0, 640.0);
    let css = base_declarations(
        &control,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
    );
    assert!(!css.contains("width:136"), "{css}");
}

/// An authored width on a labelled button is a deliberate size and has to survive.
#[test]
fn keeps_the_authored_width_of_a_labelled_button() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut control = node("button", 0.0, 160.0);
    control.rect.height = 36.0;
    control.style.insert("width".into(), "160px".into());
    control.attributes.insert("class".into(), "cta".into());
    let parent = node("div", 0.0, 640.0);
    let css = base_declarations(
        &control,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[".cta { width: 160px; }".into()],
        false,
    );
    assert!(css.contains("width:160px"), "{css}");
}

#[test]
fn preserves_svg_graphic_width_when_it_fills_its_parent() {
    let viewport = Viewport {
        width: 320,
        height: 568,
        dpr: 1.0,
    };
    let graphic = node("rect", 0.0, 120.0);
    let parent = node("svg", 0.0, 120.0);
    let mut styles = graphic.style.clone();
    let base = node("rect", 0.0, 183.0);
    normalize_viewport_width(
        &mut styles,
        &graphic,
        Some(&parent),
        &viewport,
        Some((
            &base,
            &Viewport {
                width: 1920,
                height: 1080,
                dpr: 1.0,
            },
        )),
    );
    assert_eq!(styles.get("width").map(String::as_str), Some("120px"));
}

/// An icon rendered as a `span` with `role="img"` is not a replaced element: nothing gives
/// it an intrinsic size, so its sampled width is a sample like any other. The authored
/// width is what keeps it square.
#[test]
fn keeps_the_authored_width_of_a_role_image() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut avatar = node("span", 0.0, 28.0);
    avatar.attributes.insert("role".into(), "img".into());
    let parent = node("div", 0.0, 28.0);
    let css = base_declarations(
        &avatar,
        Some(&parent),
        &viewport,
        &Default::default(),
        &["*{width:28px;}".to_string()],
        false,
    );
    assert!(css.contains("width:28px"));
}

#[test]
fn preserves_intrinsic_svg_aspect_width() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let image = node("svg", 0.0, 174.5);
    let parent = node("div", 0.0, 174.5);
    let css = base_declarations(
        &image,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
    );
    assert!(css.contains("width:174.5px"));
}
