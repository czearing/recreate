//! A state holder is reached from the subject by a combinator or by `:has()`, and Selectors
//! defines four of the former and one of the latter. The reader used to ask `closest` for the
//! holder, which walks the element and its ancestors and nothing else, so every holder left of
//! a combinator came back null and the record fell to its "the subject holds it" branch —
//! byte-identical to a rule the subject holds, with the holder itself unrecoverable.
//!
//! These tests assert on the record, because both wrong answers emit a rule that matches no
//! element in any state, so a test that only asks whether *a* relation was written passes
//! while the affordance is still missing.

use super::state_style_selector::recorded;
use super::style;
use serde_json::{Value, json};

fn element(path: &str, class: &str) -> Value {
    json!({ "path": path, "classes": [class], "computed": {} })
}

/// The state pseudo-class need not end its compound, so the holder query has to span the
/// whole compound rather than stopping where the state was removed. Cutting at the state
/// leaves `.tone ~ .sfar` as the join, which reads as a descendant of `.sinput` and matches
/// nothing.
#[test]
fn the_holder_query_spans_the_whole_compound_that_carried_the_state() {
    let mut scene = scene();
    scene["elements"][2] = json!({
        "path": "/page/switch/input", "classes": ["sinput", "tone"], "computed": {}
    });
    // The only rule in the scene, so no other rule over the same pair can stand in for it.
    scene["sheets"][0] = json!([style(
        ".sinput:focus-visible.tone ~ .sfar",
        "outline-color",
        "#8764b8"
    )]);
    let records = recorded(scene);
    assert!(
        records
            .iter()
            .any(|(target, scope, relation, _)| target == "/page/switch/far"
                && scope == "/page/switch/input"
                && relation == "preceding_sibling"),
        "a state written mid-compound still names its own compound as the holder: {records:#?}"
    );
}

/// The accessible-control shape, plus one control for each relation the pair could already
/// express. `.sind` sits immediately after `.sinput` and `.sfar` two places after it, so a
/// reader that answers `+` where the author wrote `~` picks the wrong element and a reader
/// that answers `~` where the author wrote `+` reaches one the author excluded.
fn scene() -> Value {
    let sheet = json!([
        style(".sinput:focus-visible ~ .sfar", "color", "#b4009e"),
        style(".sinput:focus-visible + .sind", "color", "#00838f"),
        style(".switch:hover > .sind", "background-color", "#107c10"),
        style(".row:hover .badge", "color", "#c50f1f"),
        style(".sind:focus-visible", "outline-color", "#0f6cbd")
    ]);
    json!({
        "elements": [
            element("/page", "page"),
            element("/page/switch", "switch"),
            element("/page/switch/input", "sinput"),
            element("/page/switch/ind", "sind"),
            element("/page/switch/far", "sfar"),
            element("/page/row", "row"),
            element("/page/row/badge", "badge")
        ],
        "matching": {},
        "sheets": [sheet]
    })
}

/// The filed defect. The native input is `position:absolute;opacity:0` and every pixel of the
/// affordance is drawn on the span beside it, so losing the holder removes the only
/// keyboard-focus affordance the control has.
#[test]
fn a_state_held_by_a_preceding_sibling_names_that_sibling_as_its_scope() {
    let records = recorded(scene());
    assert!(
        records.contains(&(
            "/page/switch/far".into(),
            "/page/switch/input".into(),
            "preceding_sibling".into(),
            ":focus-visible".into()
        )),
        "the input holds the focus and a later sibling takes the style: {records:#?}"
    );
}

/// The adjacent combinator reaches only the next sibling, so it is a different relation and
/// not a spelling of the same one. Recording both as one would emit `~` for an authored `+`
/// and paint siblings the author excluded.
#[test]
fn a_state_held_by_the_immediately_preceding_sibling_is_a_relation_of_its_own() {
    let records = recorded(scene());
    assert!(
        records.contains(&(
            "/page/switch/ind".into(),
            "/page/switch/input".into(),
            "previous_sibling".into(),
            ":focus-visible".into()
        )),
        "the adjacent sibling relation was recorded as itself: {records:#?}"
    );
}

/// A child combinator is not a descendant combinator. Collapsing it re-emits the rule as a
/// descendant one, which reaches grandchildren the author's `>` excluded.
#[test]
fn a_state_held_by_the_parent_is_not_recorded_as_held_by_any_ancestor() {
    let records = recorded(scene());
    assert!(
        records.contains(&(
            "/page/switch/ind".into(),
            "/page/switch".into(),
            "parent".into(),
            ":hover".into()
        )),
        "the child combinator survived as a relation of its own: {records:#?}"
    );
}

/// The two relations that already worked must be untouched, or the repair has traded one
/// wrong answer for another. A descendant rule stays a descendant rule even though its
/// holder happens to be the direct parent of the element it styles.
#[test]
fn the_relations_that_already_worked_are_recorded_exactly_as_before() {
    let records = recorded(scene());
    assert!(
        records.contains(&(
            "/page/row/badge".into(),
            "/page/row".into(),
            "ancestor".into(),
            ":hover".into()
        )),
        "a descendant rule whose holder is the direct parent stayed a descendant rule: {records:#?}"
    );
    assert!(
        records.contains(&(
            "/page/switch/ind".into(),
            String::new(),
            "ancestor".into(),
            ":focus-visible".into()
        )),
        "a state on the subject itself still has no scope: {records:#?}"
    );
}

/// The discriminator against the cheapest wrong fix. Both values the record could already
/// hold emit a rule matching nothing — `contained` looks inside an element the holder is not
/// inside, and the empty relation puts `:focus-visible` on a span that cannot be focused — so
/// picking the other one passes any "a relation was recorded" assertion with the ring gone.
#[test]
fn no_sibling_held_state_is_recorded_as_held_by_the_subject_or_inside_it() {
    let records = recorded(scene());
    for (target, scope, relation, _) in &records {
        if target != "/page/switch/ind" && target != "/page/switch/far" {
            continue;
        }
        if relation == "ancestor" && scope.is_empty() {
            assert_eq!(
                target, "/page/switch/ind",
                "only the rule authored on the subject itself may have no scope: {records:#?}"
            );
            continue;
        }
        assert_ne!(
            relation, "contained",
            "a sibling holder is not inside the element it styles: {records:#?}"
        );
        assert!(
            !scope.is_empty(),
            "a rule fired from another element recorded that element: {records:#?}"
        );
    }
}

/// The holder is asked of the engine, so a combinator the reader has never been taught still
/// finds its element. This join carries an intermediate compound, which no single relation
/// expresses; the pre-existing ancestor approximation is what it falls back to, and the point
/// is that the holder is still found rather than lost.
#[test]
fn a_join_carrying_an_intermediate_compound_still_finds_its_holder() {
    let mut scene = scene();
    scene["sheets"][0].as_array_mut().unwrap().push(style(
        ".switch:hover > .sinput ~ .sfar",
        "border-color",
        "#8764b8",
    ));
    let records = recorded(scene);
    assert!(
        records
            .iter()
            .any(|(target, scope, relation, _)| target == "/page/switch/far"
                && scope == "/page/switch"
                && relation == "ancestor"),
        "the holder two combinators away was located and recorded: {records:#?}"
    );
}
