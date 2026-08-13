use super::{recorded as recorded_rules, style, walk};
use serde_json::{Value, json};

/// A condition answered by the **document**. Both are re-answered by whoever views the
/// recreation: a media condition by the viewport, a container condition by the used
/// inline-size of the nearest ancestor with `container-type`, which layout produces and
/// re-produces on every resize. Spelled here rather than imported from the walk so the test
/// cannot agree with the code under test by construction.
const DOCUMENT_ANSWERED: &[(&str, &str)] = &[
    ("@media (min-width: 900px)", "(min-width: 900px)"),
    (
        "@container panelwrap (min-width: 900px)",
        "panelwrap (min-width: 900px)",
    ),
    ("@container style(--mode: wide)", "style(--mode: wide)"),
];

/// A condition answered by the **user agent**. Feature support is a property of the engine,
/// fixed for the run, and the artifact does not reproduce the engine — so re-emitting it
/// would make the recreation re-ask the viewing engine a question the capturing engine
/// already answered.
const AGENT_ANSWERED: &str = "@supports (display: grid)";

fn sheet(prelude: &str, condition: &str, inner: Value) -> Value {
    json!({ "prelude": prelude, "conditionText": condition, "rules": [inner] })
}

fn scene(sheet: Value, matching: Value) -> Value {
    json!({
        "elements": [{ "path": "/main/div", "classes": ["panel"] }],
        "matching": matching,
        "sheets": [[sheet]]
    })
}

fn recorded(scene: Value) -> Vec<String> {
    recorded_rules(&walk(scene))
}

/// The invariant, as a relation rather than an example.
///
/// Wrapping a rule in a document-answered condition must change what is recorded by exactly
/// that wrapper — never by dropping the condition and keeping the body, which publishes an
/// unconditional rule the author never wrote, and never by dropping both, which deletes the
/// branch. Stated this way it covers `@container` size and `style()` queries, `@media`, and
/// any context-sensitive grouping rule added later, where an example-based assertion on one
/// threshold would close one cell and leave the rest erased.
///
/// Crucially the relation must hold **whether or not the condition is currently satisfied**,
/// because that is precisely what an unsatisfied branch is for: the recreation is a live
/// document whose container may be resized to satisfy it.
#[test]
fn a_document_answered_condition_is_recorded_around_the_rule_it_guards() {
    for (prelude, condition) in DOCUMENT_ANSWERED {
        for satisfied in [false, true] {
            let matching = if satisfied {
                json!({ *prelude: ["/main/div"] })
            } else {
                json!({})
            };
            let rules = recorded(scene(
                sheet(prelude, condition, style(".panel", "width", "100%")),
                matching,
            ));
            assert_eq!(
                rules.len(),
                1,
                "{prelude} (satisfied={satisfied}) recorded {} rules: {rules:?}",
                rules.len()
            );
            assert!(
                rules[0].starts_with(prelude),
                "{prelude} (satisfied={satisfied}) lost its condition: {rules:?}"
            );
            assert!(
                rules[0].contains("width: 100%"),
                "{prelude} (satisfied={satisfied}) lost the rule it guards: {rules:?}"
            );
        }
    }
}

/// The converse, so widening reach does not become "carry every condition". An agent-
/// answered condition must still be evaluated now and dropped: recorded bare when it holds,
/// not recorded at all when it does not. Without this the fix would re-emit `@supports` and
/// make the artifact's rendering depend on the viewing engine's feature support rather than
/// the capturing engine's.
#[test]
fn an_agent_answered_condition_is_evaluated_and_dropped() {
    let live = recorded(scene(
        sheet(
            AGENT_ANSWERED,
            "(display: grid)",
            style(".panel", "gap", "8px"),
        ),
        json!({ AGENT_ANSWERED: ["/main/div"] }),
    ));
    assert_eq!(live.len(), 1, "{live:?}");
    assert!(
        live[0].starts_with(".panel") && live[0].contains("gap"),
        "a satisfied feature query must contribute its rule, unwrapped: {live:?}"
    );

    let dead = recorded(scene(
        sheet(
            AGENT_ANSWERED,
            "(display: grid)",
            style(".panel", "gap", "8px"),
        ),
        json!({}),
    ));
    assert!(
        dead.is_empty(),
        "an unsatisfied feature query contributed a rule no browser applied: {dead:?}"
    );
}

/// A document-answered condition inside an agent-answered one keeps the gate's verdict and
/// the carrier's text, so the two classifications compose rather than override. The failing
/// direction here is the one that publishes a fabrication: keeping the body because the gate
/// held while dropping the container condition that still guards it.
#[test]
fn a_carried_condition_inside_a_gate_keeps_both_verdicts() {
    let nested = json!({
        "prelude": AGENT_ANSWERED,
        "conditionText": "(display: grid)",
        "rules": [sheet(
            "@container panelwrap (min-width: 900px)",
            "panelwrap (min-width: 900px)",
            style(".panel", "width", "100%"),
        )]
    });
    let live = recorded(scene(
        nested.clone(),
        json!({ AGENT_ANSWERED: ["/main/div"] }),
    ));
    assert_eq!(live.len(), 1, "{live:?}");
    assert!(
        live[0].starts_with("@container panelwrap (min-width: 900px)"),
        "the gate held, so the carried container condition must survive it: {live:?}"
    );

    let dead = recorded(scene(nested, json!({})));
    assert!(
        dead.is_empty(),
        "the gate failed, so nothing inside it may be recorded: {dead:?}"
    );
}
