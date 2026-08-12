use super::authored_css::normalize;
use crate::model::{Node, Rect, Styles, WritingMode};

/// A box inside a wrapper that declares the writing mode. The box itself declares nothing,
/// which is the ordinary shape: `writing-mode` is inherited, so a page declares it once and
/// the revert-differencing baseline prunes it from every descendant's authored map. A
/// fixture that hand-placed `writing-mode` here would construct the one shape the old guard
/// could see, and would stay green through the defect it was meant to catch.
fn node(mode: WritingMode) -> Node {
    let mut node = Node {
        disabled: false,
        rtl: false,
        writing_mode: mode,
        path: "html>body>section>div".into(),
        parent: Some("html>body>section".into()),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 244.0,
            height: 111.0,
        },
        style: Styles::new(),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "box".into());
    node
}

/// The two axes exchange places under a vertical writing mode, so an authored
/// `inline-size` is the box's HEIGHT. Emitting it as `width` does not lose the value, it
/// moves it onto the other axis — and the same rule moves `block-size` back the other way,
/// so the two authored sizes land on each other's property.
#[test]
fn a_logical_size_resolves_against_the_inherited_writing_mode() {
    let node = node(WritingMode::VerticalRl);
    assert!(!node.style.contains_key("writing-mode"));
    let mut styles = Styles::new();
    styles.insert("width".into(), "148px".into());
    styles.insert("height".into(), "183px".into());
    normalize(
        &mut styles,
        &node,
        &[".box{inline-size:37%;block-size:61%;}".into()],
    );
    assert_eq!(styles["height"], "37%");
    assert_eq!(styles["width"], "61%");
}

/// The control for the case above, and the guarantee that the horizontal branch is
/// untouched. Every page that never declares a writing mode takes this path, so it must
/// keep mapping exactly as it did before the fact existed.
#[test]
fn a_logical_size_under_horizontal_text_is_unchanged() {
    let mut styles = Styles::new();
    styles.insert("width".into(), "148px".into());
    styles.insert("height".into(), "183px".into());
    normalize(
        &mut styles,
        &node(WritingMode::default()),
        &[".box{inline-size:37%;block-size:61%;}".into()],
    );
    assert_eq!(styles["width"], "37%");
    assert_eq!(styles["height"], "61%");
}

/// The defect stated as the relation that proves it: the same two authored percentages,
/// under two different writing modes, must not land on the same physical properties. This
/// is the file-level check the scene makes, at the level of the function that decides it.
#[test]
fn two_writing_modes_do_not_produce_the_same_physical_rule() {
    let rule = [".box{inline-size:37%;block-size:61%;}".to_string()];
    let resolve = |mode| {
        let mut styles = Styles::new();
        styles.insert("width".into(), "148px".into());
        styles.insert("height".into(), "183px".into());
        normalize(&mut styles, &node(mode), &rule);
        styles
    };
    assert_ne!(
        resolve(WritingMode::VerticalRl),
        resolve(WritingMode::default())
    );
}

/// The second consumer of the same fact. `suppress_derived_insets` DELETES the physical
/// edge it judges engine-derived, and every part of that judgement is written for
/// horizontal text, so acting on a vertical box removes a real anchor. Its guard was
/// always correct and was simply unable to fire, because it asked the box's own authored
/// map about a property the box inherits.
#[test]
fn the_inset_arbiter_declines_under_an_inherited_vertical_mode() {
    let mut node = node(WritingMode::VerticalLr);
    let mut styles = Styles::new();
    styles.insert("position".into(), "absolute".into());
    styles.insert("left".into(), "648px".into());
    node.style = styles.clone();
    assert!(!node.style.contains_key("writing-mode"));
    normalize(
        &mut styles,
        &node,
        &[".box{position:absolute;inset-inline-end:30%;}".into()],
    );
    assert_eq!(styles["left"], "648px");
}
