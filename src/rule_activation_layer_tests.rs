use super::{recorded, style, walk};
use serde_json::{Value, json};

/// A sheet whose cascade is decided by layers rather than by specificity.
///
/// `#box` is layered and `.box` is not, so the browser paints green: an unlayered
/// declaration outranks a layered one at every specificity. Flattened, both are unlayered
/// and the id wins, which inverts the result — the reason membership has to survive.
///
/// `.dup` declares the same text in two different layers, which is what proves the
/// recorded set is keyed by more than rule text. `@layer outer` wrapping a satisfied
/// `@media` is the nesting case. The trailing `@media` block is the positive control: it
/// is the one wrapper this walk already preserves, so it must hold in every run.
fn scene() -> Value {
    let sheet = json!([
        { "statement": "@layer base, theme;" },
        { "prelude": "@layer base", "rules": [style(".box", "background", "red")] },
        style(".box", "background", "green"),
        { "prelude": "@layer theme", "rules": [style(".dup", "color", "blue")] },
        { "prelude": "@layer base", "rules": [style(".dup", "color", "blue")] },
        {
            "prelude": "@layer outer",
            "rules": [{
                "prelude": "@media (min-width: 0px)",
                "conditionText": "(min-width: 0px)",
                "media": true,
                "rules": [style(".nested", "gap", "4px")]
            }]
        },
        {
            "prelude": "@keyframes pulse",
            "keyframes": true,
            "rules": [style("0%", "opacity", "0.25"), style("100%", "opacity", "1")]
        },
        {
            "prelude": "@media (min-width: 1px)",
            "conditionText": "(min-width: 1px)",
            "media": true,
            "rules": [style(".probe", "outline", "1px solid blue")]
        }
    ]);
    json!({
        "elements": [
            { "path": "/main/div", "classes": ["box"] },
            { "path": "/main/p", "classes": ["dup"] },
            { "path": "/main/span", "classes": ["nested"] },
            { "path": "/main/i", "classes": ["probe"] }
        ],
        "matching": {
            "@media (min-width: 0px)": ["/main/span"],
            "@media (min-width: 1px)": ["/main/i"]
        },
        "sheets": [sheet]
    })
}

fn position(rules: &[String], needle: &str) -> Option<usize> {
    rules.iter().position(|rule| rule.starts_with(needle))
}

/// A cascade layer outranks specificity outright, so unwrapping a layered rule does not
/// nudge it — it promotes it above every unlayered rule on the page. The wrapper is the
/// whole of that information and no flattened copy can carry it.
#[test]
fn a_layered_rule_keeps_the_layer_that_positions_it_in_the_cascade() {
    let rules = recorded(&walk(scene()));
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with("@layer base{") && rule.contains("background: red")),
        "lost the layer that positioned a rule in the cascade: {rules:?}"
    );
    assert!(
        !rules
            .iter()
            .any(|rule| rule.starts_with(".box") && rule.contains("background: red")),
        "promoted a layered rule to the top level: {rules:?}"
    );
}

/// The inverse. Wrapping is not free — a rule the author left unlayered must stay
/// unlayered, because that is what makes it win.
#[test]
fn an_unlayered_rule_is_not_given_a_layer_it_never_had() {
    let rules = recorded(&walk(scene()));
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with(".box") && rule.contains("background: green")),
        "invented a wrapper around an unlayered rule: {rules:?}"
    );
}

/// Layer precedence is the order in which layer names are first encountered, not the
/// order of the blocks. Re-wrapping each rule in place restores membership and can still
/// invert precedence, so the order statement has to survive and has to come first.
#[test]
fn the_layer_order_statement_still_precedes_the_layers_it_orders() {
    let rules = recorded(&walk(scene()));
    let statement = position(&rules, "@layer base, theme;")
        .unwrap_or_else(|| panic!("dropped the layer order statement: {rules:?}"));
    let block = position(&rules, "@layer base{")
        .unwrap_or_else(|| panic!("dropped the layered block: {rules:?}"));
    assert!(
        statement < block,
        "recorded a layer's rules ahead of the statement that orders it: {rules:?}"
    );
}

/// The recorded set is deduplicated on rule text. Two identical declarations in different
/// layers are different declarations — they resolve differently — so collapsing them to
/// one entry destroys membership in a way no later pass can reconstruct.
#[test]
fn identical_declarations_in_different_layers_are_recorded_once_each() {
    let rules = recorded(&walk(scene()));
    for layer in ["@layer theme{", "@layer base{"] {
        assert!(
            rules
                .iter()
                .any(|rule| rule.starts_with(layer) && rule.contains("color: blue")),
            "deduplication collapsed two layers into one: {rules:?}"
        );
    }
}

/// A layer wrapping a condition has to be rebuilt as the whole prelude stack. Collapsing
/// to one level keeps whichever wrapper the walk happened to reach last and loses the
/// other, so both the layer and the block it groups are asserted.
#[test]
fn a_layer_wrapping_a_condition_rebuilds_the_whole_prelude_stack() {
    let rules = recorded(&walk(scene()));
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with("@layer outer{") && rule.contains("gap: 4px")),
        "flattened a rule out of the layer that groups its condition: {rules:?}"
    );
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with("@layer outer{@media (min-width: 0px)")),
        "lost the layer around an authored media block: {rules:?}"
    );
}

/// A keyframes block exposes `cssRules` like a grouping rule but its children are keyframe
/// selectors, not rules the cascade resolves. Descending into one records a percentage as
/// an authored rule and re-emits each keyframe as a stylesheet of its own, splitting one
/// animation into several that each define a single stop.
///
/// The block itself must still be recorded whole. It defines a name that every
/// `animation-name` refers to and that no computed style carries, so a walk that skips it
/// along with its children leaves the animation named and undefined.
#[test]
fn a_keyframe_selector_is_not_recorded_as_an_authored_rule() {
    let rules = recorded(&walk(scene()));
    assert!(
        !rules
            .iter()
            .any(|rule| rule.starts_with("0%") || rule.starts_with("100%")),
        "walked into a block whose children are not style rules: {rules:?}"
    );
    let blocks: Vec<_> = rules
        .iter()
        .filter(|rule| rule.starts_with("@keyframes pulse"))
        .collect();
    assert_eq!(
        blocks.len(),
        1,
        "the authored keyframes block must be recorded exactly once: {rules:?}"
    );
    assert!(
        blocks[0].contains("0.25") && blocks[0].contains("100%"),
        "recorded the keyframes block without its stops: {blocks:?}"
    );
}

/// The positive control. `@media` is the one grouping wrapper this walk already keeps, so
/// it must hold in every run. If it fails, the harness is at fault and nothing above this
/// line has been tested.
#[test]
fn the_media_control_still_wraps_its_own_rule() {
    let rules = recorded(&walk(scene()));
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with("@media (min-width: 1px)") && rule.contains(".probe")),
        "regressed the wrapper this path already preserved: {rules:?}"
    );
}
