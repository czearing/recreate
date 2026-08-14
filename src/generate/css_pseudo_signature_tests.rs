//! The responsive signature builds its own bytes from the same record class_maps hashes,
//! so every collision the class identity has to avoid has a twin here. They are separated
//! because a mutation removing the slot name from one left every assertion on the other
//! green: the two encodings share a record, not an implementation.

use super::css_pseudo_identity_tests::{pseudo, span};
use crate::model::{PageState, Pseudo, Styles};

/// The same collision one level down, on the other signature. `class_maps` frames the slot
/// through the rule parts it hashes, but the responsive signature builds its own bytes, and a
/// mutation removing the slot name there left every assertion above green. Two elements alike
/// but for which slot their decoration occupies must be told apart on both paths, or the
/// responsive pass groups them and writes one element's decoration on the other's side.
#[test]
fn separates_the_slots_in_the_responsive_signature_too() {
    let signature = |suffix: &str| {
        let mut node = span(1);
        node.path = "html>body:nth-of-type(1)>span:nth-of-type(1)".into();
        node.parent = Some("html>body:nth-of-type(1)".into());
        node.pseudos
            .insert(suffix.into(), pseudo("\"MARK\"", "red"));
        let specification = crate::model::Specification {
            schema_version: 1,
            requested_url: String::new(),
            captured_url: String::new(),
            states: vec![PageState {
                nodes: vec![node],
                ..Default::default()
            }],
            interactions: Vec::new(),
            transitions: Vec::new(),
        };
        super::css_values::responsive_signatures_for(&specification, None, &Default::default())
            .into_values()
            .next()
            .expect("the node has a signature")
    };

    assert_ne!(
        signature("::before"),
        signature("::after"),
        "a leading and a trailing decoration share one responsive signature"
    );
}

/// The framed sibling had a seam of its own: it folded a decoration's payload in with no
/// terminator, so the payload ran straight into the first property name after it. A decoration
/// saying `ab` under property `c` and one saying `a` under property `bc` produced identical
/// bytes, which is the same flaw one level down.
#[test]
fn separates_a_decorations_payload_from_the_property_that_follows_it() {
    let signature = |content: &str, property: &str| {
        let mut style = Styles::new();
        style.insert(property.into(), "red".into());
        let mut node = span(1);
        node.path = "html>body:nth-of-type(1)>span:nth-of-type(1)".into();
        node.parent = Some("html>body:nth-of-type(1)".into());
        node.pseudos.insert(
            "::before".into(),
            Pseudo {
                content: content.into(),
                style,
            },
        );
        let specification = crate::model::Specification {
            schema_version: 1,
            requested_url: String::new(),
            captured_url: String::new(),
            states: vec![PageState {
                nodes: vec![node],
                ..Default::default()
            }],
            interactions: Vec::new(),
            transitions: Vec::new(),
        };
        super::css_values::responsive_signatures_for(&specification, None, &Default::default())
            .into_values()
            .next()
            .expect("the node has a signature")
    };

    assert_ne!(
        signature("ab", "c"),
        signature("a", "bc"),
        "a decoration's payload ran into the property name that followed it"
    );
}
