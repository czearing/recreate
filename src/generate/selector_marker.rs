//! Identity tokens for the elements an authored selector names.
//!
//! The generated paint class is an equivalence class over computed style: it is minted from
//! a style-and-layout signature and is deliberately many-to-one, because that is what keeps
//! the emitted sheet small. It answers "paint these elements alike", and it is the right
//! answer to that question.
//!
//! A rewritten selector asks a different question. `.theme .card` distinguishes two elements
//! the page paints identically — that is what scoping is for — so a token used to encode the
//! relationship must be injective with respect to the distinction the author drew. Borrowing
//! the paint class makes the rewritten rule reach every look-alike the author excluded.
//!
//! So identity is separated from paint. Each authored compound taking part in a rewrite gets
//! its own marker, carried as an extra class on exactly the elements that compound matches,
//! and the rewrite is expressed over markers. That holds for a lone compound too: `.subject`
//! and `.control` collapse to one paint class when the page paints them alike, so a rewrite
//! that borrowed it would apply each element's authored rule to the other. The marker is
//! derived from the compound rather than from a node, so one rule still serves every element
//! it matches and deduplication is untouched; and no marker exists unless a selector survived
//! to be rewritten, so a page without one is byte-identical.

use super::compound::matches_node;
use super::css_values::{append_class, hash};
use crate::model::Node;
use std::collections::{BTreeMap, BTreeSet};

/// The marker naming the elements this authored compound matches.
///
/// `s` separates markers from paint classes, which are the same prefix followed by hex.
pub(super) fn name(prefix: &str, compound: &str) -> String {
    format!("{prefix}s{}", &hash(compound.trim())[..8])
}

/// Carry each compound's marker on the elements it matches.
pub(super) fn apply(
    compounds: &BTreeSet<String>,
    nodes: &[Node],
    prefix: &str,
    classes: &mut BTreeMap<String, String>,
) {
    for compound in compounds {
        let marker = name(prefix, compound);
        for node in nodes.iter().filter(|node| matches_node(compound, node)) {
            append_class(classes.entry(node.path.clone()).or_default(), &marker);
        }
    }
}
