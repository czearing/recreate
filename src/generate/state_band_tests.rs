use crate::generate::project_test_support::state;
use crate::generate::state_style_maps::append;
use crate::model::{Specification, StateStyle};
use std::collections::BTreeMap;

const TARGET: &str = "html>body:nth-of-type(1)>div:nth-of-type(1)>div:nth-of-type(1)";

fn hover(declarations: &str) -> StateStyle {
    StateStyle {
        target: TARGET.into(),
        scope: None,
        relation: Default::default(),
        pseudo: Some(":hover".into()),
        target_pseudo: None,
        media: None,
        declarations: declarations.into(),
    }
}

/// One state per viewport, each carrying the state rules given for it.
fn specification(per_state: [&str; 5]) -> Specification {
    let states = [1920, 1440, 768, 390, 320]
        .iter()
        .zip(per_state)
        .map(|(width, declarations)| {
            let mut state = state(*width);
            state.state_styles = vec![hover(declarations)];
            state
        })
        .collect::<Vec<_>>();
    Specification {
        schema_version: 1,
        requested_url: states[0].url.clone(),
        captured_url: states[0].url.clone(),
        states,
        interactions: Vec::new(),
        transitions: Vec::new(),
    }
}

fn render(per_state: [&str; 5]) -> String {
    let specification = specification(per_state);
    let classes = specification
        .states
        .iter()
        .map(|_| BTreeMap::from([(TARGET.to_string(), "rTARGET".to_string())]))
        .collect::<Vec<_>>();
    let mut css = String::new();
    append(&specification, &classes, &[], &BTreeMap::new(), &mut css);
    css
}

/// A page whose state rules do not vary with width is captured once per viewport, so the same
/// rule arrives five times. Restating it inside a band declares nothing the base did not already
/// declare at every width, and the repetition is the whole of the band's content.
#[test]
fn drops_a_responsive_band_that_only_restates_the_base() {
    let css = render(["color: rgb(0, 0, 255);"; 5]);

    assert!(!css.contains("@media"), "expected no bands, got:\n{css}");
    assert_eq!(css.matches("rTARGET:hover").count(), 1, "css:\n{css}");
}

/// The inverse guard: a band that disagrees with the base carries the only copy of its own
/// declaration, so dropping it would lose the narrow-viewport value entirely.
#[test]
fn keeps_a_responsive_band_that_disagrees_with_the_base() {
    let css = render([
        "color: rgb(0, 0, 255);",
        "color: rgb(0, 0, 255);",
        "color: rgb(0, 0, 255);",
        "color: rgb(0, 0, 255);",
        "color: rgb(255, 0, 0);",
    ]);

    assert!(css.contains("@media"), "expected a band, got:\n{css}");
    assert!(css.contains("rgb(255, 0, 0)"), "css:\n{css}");
    let banded = css.split("@media").nth(1).unwrap_or_default();
    assert!(
        banded.contains("rgb(255, 0, 0)") && !banded.contains("rgb(0, 0, 255)"),
        "only the disagreeing band should survive, got:\n{css}"
    );
}

/// An interaction repaints the page under its own class names, so the same authored state rule
/// reaches those elements too. It is one authored rule, so it must reach the stylesheet once:
/// publishing it separately for each class map writes the same declarations twice and grows the
/// output by a copy per interaction without changing what any element paints.
#[test]
fn publishes_a_base_state_rule_once_for_every_class_map_that_carries_it() {
    let mut specification = specification(["color: rgb(0, 0, 255);"; 5]);
    let mut interaction_state = state(1920);
    interaction_state.state_styles = Vec::new();
    specification.interactions = vec![crate::model::Interaction {
        trigger_path: TARGET.into(),
        trigger_tag: "button".into(),
        trigger_label: "open".into(),
        trigger_occurrence: None,
        focused_path: None,
        states: vec![interaction_state],
    }];
    let base = vec![BTreeMap::from([(TARGET.to_string(), "rTARGET".to_string())]); 5];
    let interaction_classes = vec![vec![BTreeMap::from([(
        TARGET.to_string(),
        "sTARGET".to_string(),
    )])]];

    let mut css = String::new();
    append(
        &specification,
        &base,
        &interaction_classes,
        &BTreeMap::new(),
        &mut css,
    );

    assert_eq!(css.matches("rTARGET:hover").count(), 1, "css:\n{css}");
    assert_eq!(css.matches("sTARGET:hover").count(), 1, "css:\n{css}");
    assert_eq!(
        css.matches("color: rgb(0, 0, 255);").count(),
        1,
        "both class maps name one rule, so its declarations are written once:\n{css}"
    );
}

/// Bands that restate the base are dropped without disturbing the boundaries of the bands that
/// remain, because the widths that define a band come from the captured viewport list, not from
/// which neighbours happened to be emitted.
#[test]
fn dropping_a_band_does_not_move_the_boundaries_of_the_bands_that_remain() {
    let all = render([
        "color: rgb(0, 0, 255);",
        "color: rgb(1, 1, 1);",
        "color: rgb(2, 2, 2);",
        "color: rgb(3, 3, 3);",
        "color: rgb(4, 4, 4);",
    ]);
    let narrowest = all
        .split("@media")
        .last()
        .expect("four bands")
        .split('{')
        .next()
        .expect("a condition")
        .to_string();

    let sparse = render([
        "color: rgb(0, 0, 255);",
        "color: rgb(0, 0, 255);",
        "color: rgb(0, 0, 255);",
        "color: rgb(0, 0, 255);",
        "color: rgb(4, 4, 4);",
    ]);
    let remaining = sparse
        .split("@media")
        .last()
        .expect("one band")
        .split('{')
        .next()
        .expect("a condition")
        .to_string();

    assert_eq!(narrowest, remaining, "band condition changed:\n{sparse}");
}
