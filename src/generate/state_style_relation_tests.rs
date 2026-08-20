//! Which combinator an emitted state rule puts back between the holder and the subject.
//!
//! A record naming only the two elements cannot say how the author joined them, and both
//! answers a two-valued relation can give emit a rule that matches no element in any state.
//! These call the emitter directly, so they fail for the reason they name.

use super::tests::style;
use super::*;

/// A combinator puts the state holder beside the subject rather than above or inside it, and
/// each of the four names a different reach. The pair alone cannot say which, so a record that
/// carries only `scope` and `target` has to guess — and both available guesses emit a rule
/// that matches nothing: containment looks inside an element the holder is not inside, and no
/// scope at all puts the state on a decorative span that can never take it.
#[test]
fn emits_each_combinator_the_author_wrote_rather_than_a_relation_that_fits() {
    let classes = BTreeMap::from([
        ("html>body>label>input".into(), "input".into()),
        ("html>body>label>span".into(), "indicator".into()),
    ]);
    for (relation, expected) in [
        (
            crate::model::Relation::PrecedingSibling,
            ".input:focus-visible~.indicator{box-shadow: 0 0 0 1px;}",
        ),
        (
            crate::model::Relation::PreviousSibling,
            ".input:focus-visible+.indicator{box-shadow: 0 0 0 1px;}",
        ),
        (
            crate::model::Relation::Parent,
            ".input:focus-visible>.indicator{box-shadow: 0 0 0 1px;}",
        ),
        (
            crate::model::Relation::Ancestor,
            ".input:focus-visible .indicator{box-shadow: 0 0 0 1px;}",
        ),
    ] {
        let mut sibling = style(
            "html>body>label>span",
            Some(":focus-visible"),
            "box-shadow: 0 0 0 1px;",
        );
        sibling.scope = Some("html>body>label>input".into());
        sibling.relation = relation;
        let mut css = String::new();
        append(&[sibling], &classes, &BTreeMap::new(), &mut css);
        assert!(
            css.contains(expected),
            "{relation:?} emits {expected}, got {css}"
        );
        assert!(
            !css.starts_with(".indicator:focus-visible"),
            "{relation:?} put the state on the element that does not hold it: {css}"
        );
    }
}

/// The relation is part of what the rule says, so two records alike in every other field are
/// two rules. Dropping it from the group key would let one overwrite the other.
#[test]
fn two_relations_over_one_pair_are_two_rules() {
    let classes = BTreeMap::from([
        ("html>body>label>input".into(), "input".into()),
        ("html>body>label>span".into(), "indicator".into()),
    ]);
    let mut adjacent = style("html>body>label>span", Some(":hover"), "color: red;");
    adjacent.scope = Some("html>body>label>input".into());
    adjacent.relation = crate::model::Relation::PreviousSibling;
    let mut general = adjacent.clone();
    general.relation = crate::model::Relation::PrecedingSibling;
    let mut css = String::new();
    append(&[adjacent, general], &classes, &BTreeMap::new(), &mut css);
    assert!(css.contains(".input:hover+.indicator"), "{css}");
    assert!(css.contains(".input:hover~.indicator"), "{css}");
}
