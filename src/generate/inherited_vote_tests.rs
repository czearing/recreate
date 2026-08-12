//! The candidate list `inherited_value` votes over must not be the list it filtered.
//!
//! `var()` is substituted at computed-value time, after the cascade has already chosen a winner,
//! so a reference competes with a literal on equal terms. Removing references before the
//! unanimity test manufactures the agreement the test looks for, and the removed candidate is
//! systematically the higher-precedence themed override authored to beat a base literal.

use crate::generate::authored_css::Index;
use crate::generate::inherited_styles::normalize;
use crate::model::{Node, Rect, Styles};

fn node(tag: &str, class: &str, styles: &[(&str, &str)]) -> Node {
    let mut node = Node {
        writing_mode: Default::default(),
        disabled: false,
        rtl: false,
        path: tag.into(),
        parent: None,
        tag: tag.into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 16.0,
            height: 16.0,
        },
        style: Styles::from_iter(
            styles
                .iter()
                .map(|(name, value)| ((*name).into(), (*value).into())),
        ),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), class.into());
    node
}

/// What the consumer at `inherited_styles` actually writes out for `property`, given a parent
/// whose paint differs so the pruning branch cannot fire and confuse the reading.
fn emitted(child: &Node, rules: &[String], property: &str) -> Option<String> {
    let parent = node("div", "page", &[(property, "rgb(1, 1, 1)")]);
    let mut styles = child.style.clone();
    normalize(&mut styles, child, Some(&parent), rules);
    styles.get(property).cloned()
}

const THEME: &str = ":root{--accent-fill:rgb(0, 0, 255);}";
const BASE: &str = ".icon{fill:rgb(255, 0, 0);}";
const OVERRIDE: &str = ".icon.warm{fill:var(--accent-fill);}";

/// The subject. Both rules match, the reference is the later and more specific one, and the
/// engine computed blue. Reading the literal alone makes the vote unanimous for a declaration
/// that lost, and the consumer writes it over the capture.
#[test]
fn abstains_when_a_deferred_candidate_could_have_beaten_a_literal() {
    let rules = [THEME.into(), BASE.into(), OVERRIDE.into()];
    let icon = node("svg", "icon warm", &[("fill", "rgb(0, 0, 255)")]);

    assert_eq!(Index::new(&rules).inherited_value(&icon, "fill"), None);
    assert_eq!(
        emitted(&icon, &rules, "fill"),
        Some("rgb(0, 0, 255)".into()),
        "the captured post-cascade paint must survive, not merely avoid being red"
    );
}

/// The restoration control. One candidate, a literal, nothing deferred: the stage must still
/// answer, or a repair that abstains everywhere would read as a pass on the subject alone.
#[test]
fn restores_the_only_authored_literal() {
    let rules = [
        THEME.into(),
        BASE.into(),
        ".plain{fill:rgb(0, 128, 0);}".into(),
    ];
    let plain = node("svg", "plain", &[("fill", "rgb(0, 128, 0)")]);

    assert_eq!(
        Index::new(&rules).inherited_value(&plain, "fill"),
        Some("rgb(0, 128, 0)".into())
    );
}

/// The abstention control. An element whose only declaration is a reference must keep the value
/// the capture recorded and must not acquire a literal authored for a different element.
#[test]
fn does_not_fabricate_a_literal_for_an_element_declared_only_by_reference() {
    let rules = [
        THEME.into(),
        BASE.into(),
        ".themed{fill:var(--accent-fill);}".into(),
    ];
    let themed = node("svg", "themed", &[("fill", "rgb(0, 0, 255)")]);

    assert_eq!(Index::new(&rules).inherited_value(&themed, "fill"), None);
    assert_eq!(
        emitted(&themed, &rules, "fill"),
        Some("rgb(0, 0, 255)".into())
    );
}

/// Two literals that disagree. Position cannot name the winner — this index sorts by cascade
/// layer and models neither specificity nor importance — so the stage abstains and the engine's
/// own answer stands. This is why the fix does not need to prefer the last candidate: the
/// correct value is emitted either way, and preferring one end would be a guess.
#[test]
fn abstains_when_two_literals_disagree_and_still_emits_the_winner() {
    let rules = [
        ".wide{color:rgb(255, 0, 0);}".into(),
        ".tone{color:rgb(0, 0, 255);}".into(),
    ];
    let text = node("span", "wide tone", &[("color", "rgb(0, 0, 255)")]);

    assert_eq!(Index::new(&rules).inherited_value(&text, "color"), None);
    assert_eq!(
        emitted(&text, &rules, "color"),
        Some("rgb(0, 0, 255)".into())
    );
}

/// The width guard. Several rules declaring the same value are not a disagreement, so a repair
/// that abstains as soon as more than one rule mentions the property throws away the authored
/// form the capture destroyed — here `currentColor`, which no computed value can recover.
#[test]
fn restores_a_value_several_rules_state_identically() {
    let rules = [
        ".icon{fill:currentColor;}".into(),
        ".warm{fill:currentColor;}".into(),
    ];
    let icon = node("svg", "icon warm", &[("fill", "rgb(0, 0, 255)")]);

    assert_eq!(
        Index::new(&rules).inherited_value(&icon, "fill"),
        Some("currentColor".into())
    );
}

/// The scope guard. A deferred value disqualifies the property it declares, not every property
/// its rule happens to declare, so an unrelated `var()` in the same block must not suppress a
/// literal that has no competition.
#[test]
fn a_deferred_value_disqualifies_only_the_property_it_declares() {
    let rules = [
        THEME.into(),
        ".plain{fill:rgb(0, 128, 0);color:var(--accent-fill);}".into(),
    ];
    let index = Index::new(&rules);
    let plain = node("svg", "plain", &[("fill", "rgb(0, 128, 0)")]);

    assert_eq!(
        index.inherited_value(&plain, "fill"),
        Some("rgb(0, 128, 0)".into())
    );
    assert_eq!(index.inherited_value(&plain, "color"), None);
}
