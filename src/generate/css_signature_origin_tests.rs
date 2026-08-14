//! An element's identity is the rules it will receive, and the rules it will receive are the
//! rewritten ones.
//!
//! The signature that names a class was folded from the *captured* spelling of each
//! declaration, which for anything reachable by `url()` is an absolute address on the
//! capture rig's ephemeral port. That port is different on every run, so the same page
//! captured twice produced different class names, different CSS and a different JSX
//! attribute — an artifact that cannot be diffed against itself, which is the one property
//! a regression corpus is entirely built on.
//!
//! The declarations that are *emitted* have already been localised, so the mismatch is not
//! merely unstable: two elements whose captured URLs differ but whose local paths are the
//! same receive byte-identical rules under two different class names, and one duplicated
//! rule block is written for them.

use super::css_values::responsive_signatures_for;
use crate::model::{Node, PageState, Specification, Styles};
use std::collections::BTreeMap;

fn node_with(path: &str, background: &str) -> Node {
    let mut style = Styles::new();
    style.insert("background-image".into(), background.into());
    Node {
        path: path.into(),
        tag: "div".into(),
        style,
        ..Default::default()
    }
}

fn specification(nodes: Vec<Node>) -> Specification {
    Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![PageState {
            nodes,
            ..Default::default()
        }],
        interactions: Vec::new(),
        transitions: Vec::new(),
    }
}

fn assets(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(url, local)| ((*url).to_string(), (*local).to_string()))
        .collect()
}

fn signature(background: &str, assets: &BTreeMap<String, String>) -> String {
    responsive_signatures_for(
        &specification(vec![node_with("html>body", background)]),
        None,
        assets,
    )
    .into_values()
    .next()
    .expect("the node has a signature")
}

/// The subject. One page, two runs, two ephemeral ports. Both resolve to the same local
/// asset, so both elements receive the same rules and must receive the same name.
#[test]
fn one_asset_names_an_element_the_same_across_two_capture_origins() {
    assert_eq!(
        signature(
            r#"url("http://localhost:49871/plain.png")"#,
            &assets(&[("http://localhost:49871/plain.png", "/assets/abc.png")])
        ),
        signature(
            r#"url("http://localhost:50122/plain.png")"#,
            &assets(&[("http://localhost:50122/plain.png", "/assets/abc.png")])
        ),
        "an element's class depends on the capture rig's ephemeral port"
    );
}

/// The converse, so the fix is not "ignore backgrounds". Two different local assets are two
/// different rule sets and must stay two names.
#[test]
fn two_local_assets_name_two_elements_differently() {
    let map = assets(&[
        ("http://rig.test/a.png", "/assets/aaa.png"),
        ("http://rig.test/b.png", "/assets/bbb.png"),
    ]);
    assert_ne!(
        signature(r#"url("http://rig.test/a.png")"#, &map),
        signature(r#"url("http://rig.test/b.png")"#, &map),
    );
}

/// A declaration with no `url()` in it is folded exactly as before, so every page without
/// assets keeps the names it already had.
#[test]
fn a_value_without_a_url_is_unaffected_by_the_asset_map() {
    assert_eq!(
        signature(
            "none",
            &assets(&[("http://rig.test/a.png", "/assets/aaa.png")])
        ),
        signature("none", &BTreeMap::new()),
    );
}

/// Pseudo-element decoration reaches the same signature by a different path, so it needs the
/// same localisation or a `::before` background reintroduces the drift on its own.
#[test]
fn a_pseudo_background_is_localised_in_the_signature_too() {
    let pseudo_node = |origin: &str| {
        let mut node = node_with("html>body", "none");
        node.pseudos.insert(
            "::before".into(),
            crate::model::Pseudo {
                content: "\"\"".into(),
                style: {
                    let mut style = Styles::new();
                    style.insert(
                        "background-image".into(),
                        format!(r#"url("{origin}/plain.png")"#),
                    );
                    style
                },
            },
        );
        node
    };
    let signature_for = |origin: &str| {
        responsive_signatures_for(
            &specification(vec![pseudo_node(origin)]),
            None,
            &assets(&[(&format!("{origin}/plain.png"), "/assets/abc.png")]),
        )
        .into_values()
        .next()
        .expect("the node has a signature")
    };
    assert_eq!(
        signature_for("http://localhost:49871"),
        signature_for("http://localhost:50122"),
        "a decorated element's class depends on the capture rig's ephemeral port"
    );
}
