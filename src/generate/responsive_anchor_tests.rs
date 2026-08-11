//! An anchor is a parameter, not an effect, and the state that uses it need not be the
//! state that declares it. `transform-origin` percentages resolve against the border box
//! and the capture records used pixels, so an authored `top left` and an undeclared
//! origin arrive spelled identically; whether one was authored is readable only from
//! whether the authored stage above replaced the sample. The emitter used to guess, with
//! a companion test asked of the resting declarations: keep the anchor if something is
//! transformed *now*. An anchor whose transform arrives from `:hover` is inert where it
//! is declared and load-bearing where it is used, and state records are deltas that never
//! restate it, so the guess erased the only copy. At rest both outputs render identically.

use super::*;
use crate::model::{Node, Styles, Viewport};
use std::collections::BTreeMap;

/// Splits emitted `name:value;` text so a property can be asserted by name rather than by
/// substring, which would let `perspective-origin` satisfy a test written for `origin`.
fn declared(css: &str) -> BTreeMap<&str, &str> {
    css.split(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .map(|(name, value)| (name.trim(), value.trim()))
        .collect()
}

/// A 120x60 tile whose captured anchor is the sample every such box produces.
fn tile(class: &str, extra: &[(&str, &str)]) -> Node {
    let mut tile = super::tests::node("div", 0.0, 120.0);
    tile.attributes.insert("class".into(), class.into());
    tile.style.insert("height".into(), "60px".into());
    tile.style
        .insert("transform-origin".into(), "0px 0px".into());
    for (name, value) in extra {
        tile.style.insert((*name).into(), (*value).into());
    }
    tile
}

fn emit(node: &Node, rules: &[String]) -> String {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    base_declarations(node, None, &viewport, &Default::default(), rules, false)
}

/// The authored sheet of the reproduction scene: one anchor on the shared class, a
/// resting transform on the control only, and the transform both tiles receive on hover.
fn scene() -> Vec<String> {
    vec![
        ".tile { width: 120px; height: 60px; transform-origin: top left; }".into(),
        ".control { transform: translateX(0px); }".into(),
        ".tile:hover { transform: scale(1.5); }".into(),
    ]
}

/// The subject. Nothing transforms it at rest, and the hover rule that does is recorded
/// as a delta which never restates the anchor — so deleting it here deletes it from the
/// artifact entirely, and the hover scales from the box centre instead of the corner.
#[test]
fn keeps_an_anchor_whose_transform_arrives_only_in_another_state() {
    let css = emit(&tile("tile subject", &[]), &scene());
    assert_eq!(
        declared(&css).get("transform-origin"),
        Some(&"top left"),
        "the only record of the authored anchor was deleted: {css}"
    );
}

/// The control twin, which separates "the anchor is kept" from "the stage never ran".
#[test]
fn keeps_an_anchor_beside_a_resting_transform() {
    let css = emit(
        &tile("tile control", &[("transform", "matrix(1, 0, 0, 1, 0, 0)")]),
        &scene(),
    );
    assert_eq!(declared(&css).get("transform-origin"), Some(&"top left"));
}

/// The twins author the identical declaration on the identical tag, so the emitted anchor
/// must be identical. Any difference between them is decided by what else happens to be
/// declared at rest, which is exactly the defect.
#[test]
fn emits_the_same_anchor_for_twins_that_author_the_same_declaration() {
    let rules = scene();
    let subject = emit(&tile("tile subject", &[]), &rules);
    let control = emit(
        &tile("tile control", &[("transform", "matrix(1, 0, 0, 1, 0, 0)")]),
        &rules,
    );
    assert_eq!(
        declared(&subject).get("transform-origin"),
        declared(&control).get("transform-origin")
    );
}

/// The guard's own job, and the reason the pruning stage exists. An anchor nobody wrote is
/// the box centre restated in pixels, which the recreation recomputes from the box it
/// already reproduces.
#[test]
fn drops_a_sampled_anchor_the_author_never_wrote() {
    let css = emit(&tile("plain", &[]), &[]);
    assert_eq!(declared(&css).get("transform-origin"), None, "{css}");
}

/// The same sample, beside a resting transform. The old predicate kept it, so every
/// transformed element carried a pixel pair measured from its captured box — a percentage
/// anchor frozen at one viewport, which is the defect the size stages exist to prevent.
#[test]
fn drops_a_sampled_anchor_beside_a_resting_transform() {
    let css = emit(
        &tile("plain", &[("transform", "matrix(1, 0, 0, 1, 0, 0)")]),
        &[],
    );
    assert_eq!(
        declared(&css).get("transform-origin"),
        None,
        "emitted a measurement of the captured box: {css}"
    );
}

/// The same sample, beside an animation. This was the one deferred-effect case the old
/// predicate answered, and answering it kept the measurement rather than the anchor.
#[test]
fn drops_a_sampled_anchor_beside_an_animation() {
    let css = emit(&tile("plain", &[("animation-name", "pulse")]), &[]);
    assert_eq!(declared(&css).get("transform-origin"), None, "{css}");
}

/// The authored spelling is what survives a viewport change; the sampled pixels are what
/// the captured box happened to be. A fluid anchor emitted as pixels is right at exactly
/// one width.
#[test]
fn emits_the_authored_anchor_rather_than_the_pixels_it_sampled() {
    let mut node = tile("panel", &[]);
    node.style
        .insert("transform-origin".into(), "120px 30px".into());
    let css = emit(
        &node,
        &[".panel { transform-origin: right center; }".into()],
    );
    assert_eq!(
        declared(&css).get("transform-origin"),
        Some(&"right center"),
        "froze a fluid anchor at the captured width: {css}"
    );
}

/// The second property with the identical shape. A 3D card flip declares `perspective`
/// only in the state that flips, so an anchor judged against the resting declarations is
/// lost for the same reason and by the same line.
#[test]
fn keeps_a_perspective_origin_whose_perspective_arrives_only_in_another_state() {
    let mut node = tile("card", &[]);
    node.style
        .insert("perspective-origin".into(), "60px 30px".into());
    let css = emit(
        &node,
        &[
            ".card { perspective-origin: top right; }".into(),
            ".card:hover { perspective: 800px; }".into(),
        ],
    );
    assert_eq!(
        declared(&css).get("perspective-origin"),
        Some(&"top right"),
        "{css}"
    );
}

/// And its guard: an unauthored perspective origin is still the box centre in pixels.
#[test]
fn drops_a_sampled_perspective_origin() {
    let mut node = tile("plain", &[]);
    node.style
        .insert("perspective-origin".into(), "60px 30px".into());
    node.style.insert("perspective".into(), "800px".into());
    let css = emit(&node, &[]);
    assert_eq!(declared(&css).get("perspective-origin"), None, "{css}");
}

/// A pseudo-element's declarations reach the emitter exactly as captured — no stage
/// replaces a sample there, because the authored index holds no pseudo-element rules — so
/// every anchor on that path is a measurement and none of them may be emitted.
#[test]
fn drops_every_anchor_from_a_pseudo_element_style() {
    let style = Styles::from([
        ("content".into(), "\"\"".into()),
        ("transform".into(), "matrix(2, 0, 0, 2, 0, 0)".into()),
        ("transform-origin".into(), "8px 8px".into()),
        ("perspective-origin".into(), "8px 8px".into()),
    ]);
    let css = output_declarations(&style, &Default::default());
    let declared = declared(&css);
    assert_eq!(declared.get("transform-origin"), None, "{css}");
    assert_eq!(declared.get("perspective-origin"), None, "{css}");
    assert_eq!(declared.get("transform"), Some(&"matrix(2, 0, 0, 2, 0, 0)"));
}
