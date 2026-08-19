//! A conditional block decides properties it never names.
//!
//! `var()` substitutes at computed-value time, so a block that publishes a token overrides
//! whatever longhand reads it — and that longhand is declared in a different, unconditional
//! rule. The authored text of the conditional block and the set of properties it decides are
//! disjoint in general, which is why no case here lets the stage read either one off the
//! other: the engine's answer arrives on the node and the authored text is only rewritten.

use super::{credited, emitted, emitted_sweep, node, restored};
use crate::model::Node;

/// The chain the scene authors. A container rather than a viewport, so no sampled width can
/// restate either arm and the measurement is the only evidence there is.
const CARD: &str = "@container (max-width: 300px)";

/// The arm the engine read while the page was open, with the condition withdrawn.
fn measured(mut node: Node, properties: &[(&str, &str)]) -> Node {
    node.condition_base = properties
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect();
    node
}

/// One arm of the scene: a longhand that reads a token unconditionally, and a conditional
/// block that republishes the token.
fn token_scene(subject: &str, block: &str) -> Vec<String> {
    vec![
        format!(".{subject} {{ padding-left: var(--pad); }}"),
        format!("{CARD}{{.{subject} {{ {block} }}}}"),
    ]
}

/// The filed defect. The block names only the token, so a candidate set read off its text is
/// empty and the override stayed baked at every container width.
#[test]
fn publishes_the_base_arm_of_an_override_a_token_carried() {
    let node = measured(
        credited(
            node("arm-b", &[("padding-left", "62px")]),
            CARD,
            &["padding-left"],
        ),
        &[("padding-left", "5px")],
    );
    let captured = token_scene("arm-b", "--pad: 62px;");

    assert_eq!(restored(&node, &captured)["padding-left"], "5px");
    assert!(
        emitted(&node, &captured)
            .iter()
            .any(|rule| rule.contains("padding-left:62px")),
        "the band must carry the override the class no longer bakes: {:?}",
        emitted(&node, &captured)
    );
}

/// The control that failed. This block does name a property, so it passes every eligibility
/// test the old stage applied — and the property it names is not the one it decided.
#[test]
fn publishes_the_base_arm_a_block_naming_only_another_property_decided() {
    let node = measured(
        credited(
            node(
                "arm-c",
                &[("padding-left", "63px"), ("letter-spacing", "3px")],
            ),
            CARD,
            &["letter-spacing", "padding-left"],
        ),
        &[("padding-left", "6px")],
    );
    let mut captured = token_scene("arm-c", "--pad: 63px; letter-spacing: 3px;");
    captured[0] = ".arm-c { padding-left: var(--pad); letter-spacing: normal; }".into();

    let styles = restored(&node, &captured);
    assert_eq!(styles["padding-left"], "6px");
    assert_eq!(styles["letter-spacing"], "normal");
    let rules = emitted(&node, &captured);
    assert!(
        rules.iter().any(|rule| rule.contains("padding-left:63px")),
        "{rules:?}"
    );
}

/// The token is published on an ancestor and read on a descendant. Custom properties inherit,
/// so the deciding rule names neither this node nor this property, and nothing the emitter
/// can rewrite for this node mentions the condition at all — the band has to be synthesized
/// on the node's own class or the override reaches no file.
#[test]
fn synthesizes_a_band_for_a_chain_no_rule_of_this_node_carries() {
    let node = measured(
        credited(
            node("reader", &[("padding-left", "62px")]),
            CARD,
            &["padding-left"],
        ),
        &[("padding-left", "5px")],
    );
    let captured = vec![
        ".reader { padding-left: var(--pad); }".to_string(),
        format!("{CARD}{{.frame {{ --pad: 62px; }}}}"),
    ];

    assert_eq!(restored(&node, &captured)["padding-left"], "5px");
    assert_eq!(
        emitted(&node, &captured),
        vec![format!("{CARD}{{.card{{padding-left:62px;}}}}")]
    );
}

/// A condition holds one arm at a time, and the base width holds one of them, so the other
/// arm of a breakpoint the base width falsifies appears in no reading that state took. The
/// sweep measures it and nothing else does — and the viewport bands are not its carrier,
/// being quantised to the widths the capture sampled, so an author's breakpoint that falls
/// between two samples is wrong for every width in between. A token makes the case ordinary
/// rather than exotic: `:root` republishes the token on both sides, so each width sees a
/// different override of a longhand neither block names.
#[test]
fn bands_a_breakpoint_the_base_width_falsifies_from_the_width_that_measured_it() {
    let wide = measured(
        credited(
            node("box", &[("width", "24px")]),
            "@media (min-width: 701px)",
            &["width"],
        ),
        &[("width", "4px")],
    );
    let narrow = measured(
        credited(
            node("box", &[("width", "12px")]),
            "@media (max-width: 700px)",
            &["width"],
        ),
        &[("width", "4px")],
    );
    let captured = vec![
        ".box { width: var(--changed, 4px); }".to_string(),
        "@media (min-width: 701px){:root{--changed: 24px;}}".to_string(),
        "@media (max-width: 700px){:root{--changed: 12px;}}".to_string(),
    ];

    assert_eq!(restored(&wide, &captured)["width"], "4px");
    assert_eq!(
        emitted_sweep(&[wide, narrow], &captured),
        vec![
            "@media (max-width: 700px){.card{width:12px;}}".to_string(),
            "@media (min-width: 701px){.card{width:24px;}}".to_string(),
        ]
    );
}

#[path = "authored_conditions_base_arm_band_shape_tests.rs"]
mod shape;
