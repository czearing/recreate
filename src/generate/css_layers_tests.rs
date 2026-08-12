use super::{Order, peel};
use crate::generate::authored_css::Index;
use crate::model::{Node, Rect, Styles};

fn rules(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_string()).collect()
}

fn box_node() -> Node {
    let mut node = Node {
        disabled: false,
        rtl: false,
        path: "div".into(),
        parent: None,
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 120.0,
        },
        style: Styles::new(),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "box".into());
    node.attributes.insert("id".into(), "box".into());
    node
}

/// A layered rule still has to reach the index. Rule text beginning with `@` is skipped as
/// a conditional group, so restoring the wrapper without peeling it would trade layer
/// erasure for outright rule loss — worse than the defect, and worst on the pages that
/// author everything inside a layer.
#[test]
fn a_layered_declaration_is_still_indexed() {
    let rules = rules(&["@layer base{.box { padding: 8px; }}"]);
    let index = Index::new(&rules);
    assert_eq!(
        index.authored_value(&box_node(), "padding"),
        Some("8px".into()),
        "a layer wrapper hid the declaration it groups"
    );
}

/// The defect this whole item is about, measured where it decides an emitted value. An
/// unlayered declaration beats a layered one at every specificity, so the id rule loses to
/// the class rule — the reverse of what specificity alone would say.
#[test]
fn an_unlayered_declaration_outranks_a_layered_one_of_higher_specificity() {
    let rules = rules(&[
        "@layer base;",
        ".box { color: green; }",
        "@layer base{#box { color: red; }}",
    ]);
    let index = Index::new(&rules);
    assert_eq!(
        index.authored_value(&box_node(), "color"),
        Some("green".into()),
        "a layered rule was promoted above the unlayered rule that beat it"
    );
}

/// Sheet order is not layer order. `theme` is declared after `base` by the order
/// statement, so it wins even though its block appears first in the sheet — which is
/// precisely what re-wrapping each rule in place cannot restore on its own.
#[test]
fn layer_order_follows_first_declaration_not_block_position() {
    let rules = rules(&[
        "@layer base, theme;",
        "@layer theme{.box { color: blue; }}",
        "@layer base{.box { color: red; }}",
    ]);
    let index = Index::new(&rules);
    assert_eq!(
        index.authored_value(&box_node(), "color"),
        Some("blue".into()),
        "ranked layers by block position instead of by first declaration"
    );
}

/// A nested layer is positioned by its parent first. `framework.late` sits under a layer
/// declared before `app`, so nothing inside it can outrank an `app` declaration however
/// late the inner name was seen.
#[test]
fn a_nested_layer_is_positioned_by_its_parent_first() {
    let rules = rules(&[
        "@layer framework, app;",
        "@layer app{.box { color: green; }}",
        "@layer framework{@layer late{.box { color: red; }}}",
    ]);
    let index = Index::new(&rules);
    assert_eq!(
        index.authored_value(&box_node(), "color"),
        Some("green".into()),
        "an inner layer name outranked the outer layer that contains it"
    );
}

/// The unit the ranking rests on. A path is only a layer path when `@layer` introduces a
/// block; an order statement and a lookalike at-rule are neither.
#[test]
fn peel_separates_a_layer_path_from_the_rule_it_wraps() {
    assert_eq!(
        peel("@layer a{@layer b{.x { gap: 1px; }}}"),
        (Some("a.b".into()), ".x { gap: 1px; }")
    );
    assert_eq!(peel(".x { gap: 1px; }"), (None, ".x { gap: 1px; }"));
    assert_eq!(peel("@layer a, b;"), (None, "@layer a, b;"));
    assert_eq!(
        peel("@media (min-width: 1px){.x { gap: 1px; }}"),
        (None, "@media (min-width: 1px){.x { gap: 1px; }}")
    );
}

/// An order statement names layers that no block ever opens, and a block may open a layer
/// no statement named. Both register, and the first registration is the one that ranks.
#[test]
fn an_order_statement_ranks_layers_ahead_of_the_blocks_that_open_them() {
    let order = Order::new(&rules(&[
        "@layer reset, base;",
        "@layer base{.x { gap: 1px; }}",
        "@layer late{.y { gap: 2px; }}",
    ]));
    assert!(order.position(Some("reset")) < order.position(Some("base")));
    assert!(order.position(Some("base")) < order.position(Some("late")));
    assert!(
        order.position(Some("late")) < order.position(None),
        "an unlayered rule must outrank every layer"
    );
}
