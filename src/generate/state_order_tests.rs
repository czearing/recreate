//! Order is a value. Two declarations that reach one element with equal importance, origin and
//! specificity are decided by which appears last, so consolidating rules that share a
//! declaration block is only sound while it leaves every affected element's last word intact.

use super::*;

fn hover(target: &str, declarations: &str) -> StateStyle {
    StateStyle {
        target: target.into(),
        scope: None,
        relation: Default::default(),
        pseudo: Some(":hover".into()),
        target_pseudo: None,
        media: None,
        declarations: declarations.into(),
    }
}

const ALPHA: &str = "html>body>span:nth-of-type(1)";
const BETA: &str = "html>body>span:nth-of-type(2)";

fn pair() -> BTreeMap<String, String> {
    BTreeMap::from([
        (ALPHA.to_string(), "alpha".to_string()),
        (BETA.to_string(), "beta".to_string()),
    ])
}

/// The index of the last emitted rule whose selector list contains `selector` and whose body
/// carries `value` — the position at which that value becomes the element's last word.
fn last_win(css: &str, selector: &str, value: &str) -> Option<usize> {
    css.lines()
        .enumerate()
        .filter(|(_, line)| {
            let Some((selectors, body)) = line.split_once('{') else {
                return false;
            };
            selectors.split(',').any(|each| each == selector) && body.contains(value)
        })
        .map(|(index, _)| index)
        .last()
}

/// The reported defect. A restatement of an earlier block must not be folded backwards past a
/// rule it was authored to override.
///
/// Authored: beta blue, alpha red, alpha blue. The third rule's block matches the first, so a
/// consolidator keyed on declarations alone merges it into the first group and `.alpha` loses
/// to the red rule that now follows it. Every emitted byte is authored and correct; only the
/// relative position of two rules is wrong.
#[test]
fn keeps_a_restated_block_after_the_rule_it_overrides() {
    let mut css = String::new();
    append(
        &[
            hover(BETA, "color: rgb(0, 0, 255);"),
            hover(ALPHA, "color: rgb(255, 0, 0);"),
            hover(ALPHA, "color: rgb(0, 0, 255);"),
        ],
        &pair(),
        &BTreeMap::new(),
        &mut css,
    );

    let blue = last_win(&css, ".alpha:hover", "rgb(0, 0, 255)").expect("alpha keeps its blue rule");
    let red = last_win(&css, ".alpha:hover", "rgb(255, 0, 0)").expect("alpha keeps its red rule");
    assert!(
        blue > red,
        "the restated blue rule was folded back before the red rule it overrides:\n{css}"
    );
}

/// Moving a merged group to the position of the rule it just absorbed is not the repair, and
/// this is the case that proves it. A group holds several selectors, so relocating it moves
/// every one of them, including selectors whose authored position was earlier.
///
/// Authored: beta blue, beta red, alpha blue. `.beta` must stay red and `.alpha` must be blue.
/// Merging the third rule into the first group is safe precisely because nothing between them
/// mentions `.alpha`; relocating that group to the end would hand `.beta` back to blue.
#[test]
fn does_not_drag_an_earlier_selector_forward_when_a_later_one_joins_its_group() {
    let mut css = String::new();
    append(
        &[
            hover(BETA, "color: rgb(0, 0, 255);"),
            hover(BETA, "color: rgb(255, 0, 0);"),
            hover(ALPHA, "color: rgb(0, 0, 255);"),
        ],
        &pair(),
        &BTreeMap::new(),
        &mut css,
    );

    let beta_red = last_win(&css, ".beta:hover", "rgb(255, 0, 0)").expect("beta keeps its red");
    let beta_blue = last_win(&css, ".beta:hover", "rgb(0, 0, 255)").expect("beta keeps its blue");
    assert!(
        beta_red > beta_blue,
        "beta's authored last word was red, but a group it belongs to moved past it:\n{css}"
    );
    assert!(
        last_win(&css, ".alpha:hover", "rgb(0, 0, 255)").is_some(),
        "alpha lost its only rule:\n{css}"
    );
}

/// The inverse guard. Consolidation is a real optimisation and must survive: when nothing
/// between two rules can disagree with either, one rule carries both selectors.
#[test]
fn still_merges_when_nothing_between_the_rules_conflicts() {
    let mut css = String::new();
    append(
        &[
            hover(BETA, "color: rgb(0, 0, 255);"),
            hover(BETA, "letter-spacing: 2px;"),
            hover(ALPHA, "color: rgb(0, 0, 255);"),
        ],
        &pair(),
        &BTreeMap::new(),
        &mut css,
    );

    assert_eq!(
        css.matches("rgb(0, 0, 255)").count(),
        1,
        "two rules that cannot disagree were not consolidated:\n{css}"
    );
    assert!(
        css.contains(".alpha:hover,.beta:hover{color: rgb(0, 0, 255);}"),
        "{css}"
    );
}

/// An intervening rule that reaches the same element but declares nothing in common cannot
/// change which value wins, so it must not block consolidation. This pins the conflict test to
/// the property actually at stake rather than to the selector alone, which would be the lazy
/// over-approximation that trades correctness for stylesheet bloat.
#[test]
fn a_disjoint_intervening_rule_does_not_block_consolidation() {
    let mut css = String::new();
    append(
        &[
            hover(BETA, "color: rgb(0, 0, 255);"),
            hover(ALPHA, "letter-spacing: 2px;"),
            hover(ALPHA, "color: rgb(0, 0, 255);"),
        ],
        &pair(),
        &BTreeMap::new(),
        &mut css,
    );

    assert_eq!(
        css.matches("rgb(0, 0, 255)").count(),
        1,
        "an intervening rule sharing no property blocked a safe merge:\n{css}"
    );
}

/// The order invariant has to hold across the successive `collect` passes that
/// `append_inherited` makes into one group vector, not merely within a single pass.
#[test]
fn holds_the_order_invariant_across_inherited_passes() {
    let styles = [
        hover(BETA, "color: rgb(0, 0, 255);"),
        hover(ALPHA, "color: rgb(255, 0, 0);"),
        hover(ALPHA, "color: rgb(0, 0, 255);"),
    ];
    let changed = BTreeMap::from([
        (ALPHA.to_string(), "alpha2".to_string()),
        (BETA.to_string(), "beta2".to_string()),
    ]);
    let mut css = String::new();
    append_inherited(
        &styles,
        &pair(),
        &[(&[], &changed)],
        &BTreeMap::new(),
        &mut css,
    );

    for selector in [".alpha:hover", ".alpha2:hover"] {
        let blue = last_win(&css, selector, "rgb(0, 0, 255)").expect("blue survives");
        let red = last_win(&css, selector, "rgb(255, 0, 0)").expect("red survives");
        assert!(blue > red, "{selector} resolved to red:\n{css}");
    }
}
