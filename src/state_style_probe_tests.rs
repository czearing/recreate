//! How a state rule's selector is handed back to the engine, and what happens to the members
//! of a list the stage cannot honour.
//!
//! Both questions are asked of the same scripted CSSOM the sibling file drives, so a change to
//! the reader that satisfies one and breaks the other cannot pass.

use super::state_style_selector::{recorded, scene};
use super::{style, walk};
use serde_json::json;
/// A conditional rule is kept or dropped by asking the browser which elements a probe built
/// from its selector reaches. The probe is the selector with the states and the generated
/// boxes taken off it, so building it by cutting the text apart hands the engine a fragment:
/// the rule then measures whatever that fragment happens to match, or nothing at all.
#[test]
fn a_conditional_rule_is_probed_through_the_selector_the_author_wrote() {
    let mut scene = scene();
    scene["sheets"][0] = json!([{
        "prelude": "@supports (display: grid)",
        "conditionText": "(display: grid)",
        "rules": [style(
            ".ring:where(:focus-visible,[data-activedescendant-focusvisible])::after",
            "content",
            "\"!\""
        )]
    }]);
    scene["matching"] = json!({ "@supports (display: grid)": ["/main/ring"] });
    let rules = walk(scene)["cssRules"]
        .as_array()
        .expect("the walk records authored rules")
        .iter()
        .map(|rule| rule["text"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(
        rules.iter().any(|text| text.contains(".ring")),
        "the rule the probe reached was recorded: {rules:#?}"
    );
}
/// A selector list is a logical OR evaluated per member, so a member this stage cannot honour
/// is skipped and its siblings survive. Restoring the rule and then vetoing it on one member
/// would satisfy a "the rule was recorded" check and still lose the declaration.
#[test]
fn a_member_the_stage_cannot_honour_does_not_take_its_siblings_with_it() {
    let mut scene = scene();
    let sheet = scene["sheets"][0].as_array_mut().unwrap();
    // Reduces to nothing: a bare state conditions the whole document rather than an element.
    sheet.push(style(":hover, .plain:hover", "color", "#000000"));
    // Does not parse: the engine answers a fragment with a throw and no elements.
    sheet.push(style(
        ".plain:active, .broken:not(:hover",
        "color",
        "#111111",
    ));
    let records = recorded(scene);
    for state in [":hover", ":active"] {
        assert!(
            records.contains(&(
                "/main/plain".into(),
                String::new(),
                "ancestor".into(),
                state.into()
            )),
            "the honourable member survived its sibling at {state}: {records:#?}"
        );
    }
}
