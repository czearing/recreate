//! What the emitter may delete when it asks the authored sheet for a size.
//!
//! A capture reads rule text through CSSOM, which reserialises a complete set of longhands
//! back into the shorthand that gathers them — so a sheet spelling `flex-grow`, `flex-shrink`
//! and `flex-basis` separately arrives here as `flex`. Every size the author wrote through a
//! shorthand therefore reaches the emitter under a name that is not the property's own, and a
//! reader matching on the name alone reports the author as having declared nothing.

use super::node;
use crate::generate::authored_css::Authored;
use crate::generate::responsive::base_declarations;
use crate::generate::shorthand::Shorthands;
use crate::model::{Node, Viewport};

fn divisions(shares: &[(&str, &[(&str, &str)])]) -> Shorthands {
    shares
        .iter()
        .map(|(block, parts)| {
            (
                (*block).to_string(),
                parts
                    .iter()
                    .map(|(name, share)| ((*name).to_string(), (*share).to_string()))
                    .collect(),
            )
        })
        .collect()
}

fn item(class: &str, style: &[(&str, &str)]) -> Node {
    let mut item = node("div", 0.0, 679.0);
    item.attributes.insert("class".into(), class.into());
    for (name, value) in style {
        item.style.insert((*name).into(), (*value).into());
    }
    item
}

fn emitted(item: &Node, rules: &[&str], shorthands: &Shorthands) -> String {
    let viewport = Viewport {
        width: 1200,
        height: 800,
        dpr: 1.0,
    };
    let parent = node("div", 0.0, 1200.0);
    let rules: Vec<String> = rules.iter().map(|rule| (*rule).to_string()).collect();
    base_declarations(
        item,
        Some(&parent),
        &viewport,
        &Default::default(),
        Authored {
            rules: &rules,
            shorthands,
        },
        false,
    )
}

/// The three value forms a `flex` shorthand gives its basis: an absolute length, a percentage,
/// and the `0%` an omitted component produces. Each is the engine's own division of the block,
/// so none of them is a guess this stage made about the shorthand's grammar.
#[test]
fn a_size_the_author_spelled_only_through_a_shorthand_survives() {
    for (value, spelling) in [
        ("320px", "flex: 1 1 320px"),
        ("37%", "flex: 1 1 37%"),
        ("0%", "flex: 1"),
    ] {
        let block = format!("display: flex; {spelling};");
        let rule = format!(".item {{ {block} }}");
        let shorthands = divisions(&[(
            block.as_str(),
            &[
                ("flex-grow", "1"),
                ("flex-shrink", "1"),
                ("flex-basis", value),
            ],
        )]);
        let css = emitted(
            &item(
                "item",
                &[("display", "flex"), ("flex-grow", "1"), ("flex-basis", value)],
            ),
            &[&rule],
            &shorthands,
        );
        assert!(
            css.contains(&format!("flex-basis:{value}")),
            "{spelling}: {css}"
        );
    }
}

/// The generalising clause, named for no property: a size is deleted only where the author
/// declared none. A block the divider cannot read is a failed READ, and reporting it as an
/// absent DECLARATION publishes the initial value in place of one the source never took.
///
/// `grid-template-columns` stands in for `flex-basis` here deliberately — the invariant is a
/// property of the stage, so a second member of the same list must obey it too.
#[test]
fn a_size_declared_through_a_value_the_divider_cannot_read_is_kept() {
    let block = "display: grid; grid-template: auto / var(--columns);";
    let rule = format!(".panel {{ {block} }}");
    let shorthands = divisions(&[(block, &[("grid-template-columns", "")])]);
    let css = emitted(
        &item(
            "panel",
            &[
                ("display", "grid"),
                ("grid-template-columns", "120px 200px"),
            ],
        ),
        &[&rule],
        &shorthands,
    );
    assert!(css.contains("grid-template-columns:120px 200px"), "{css}");
}

/// The other half of the same rule, and the reason it cannot be satisfied by keeping
/// everything: a size no rule declares is a measurement of one viewport, and emitting it
/// freezes the box at the width the capture happened to see. A basis the sample carries and
/// the unconditional cascade does not state is such a measurement — it was authored inside a
/// condition, which the recreation re-emits as its own band, so baking it into the base rule
/// applies one band's answer at every width.
#[test]
fn a_size_no_rule_declares_is_still_dropped() {
    let block = "display: flex;";
    let rule = format!(".fluid {{ {block} }}");
    let shorthands = divisions(&[]);
    let mut fluid = item("fluid", &[("display", "flex"), ("flex-grow", "1")]);
    fluid.style.insert("height".into(), "44px".into());
    fluid.style.insert("flex-basis".into(), "320px".into());
    let css = emitted(&fluid, &[&rule], &shorthands);
    assert!(!css.contains("width:679px"), "{css}");
    assert!(!css.contains("height:44px"), "{css}");
    assert!(!css.contains("flex-basis"), "{css}");
}

/// A division names the longhands its block sets and no others, so a size the same block
/// leaves alone stays a sample and is still dropped. Without this the repair degrades into
/// "any block mentioning a shorthand keeps every size", which is the pin it exists to prevent.
#[test]
fn a_shorthand_keeps_only_the_sizes_its_division_names() {
    let block = "display: flex; flex: 1 1 320px;";
    let rule = format!(".row {{ {block} }}");
    let shorthands = divisions(&[(
        block,
        &[
            ("flex-grow", "1"),
            ("flex-shrink", "1"),
            ("flex-basis", "320px"),
        ],
    )]);
    let mut row = item(
        "row",
        &[("display", "flex"), ("flex-basis", "320px")],
    );
    row.style.insert("min-width".into(), "180px".into());
    let css = emitted(&row, &[&rule], &shorthands);
    assert!(css.contains("flex-basis:320px"), "{css}");
    assert!(!css.contains("min-width:180px"), "{css}");
}
