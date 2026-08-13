//! The record of one generated box: the reason it exists and the declarations it carries.

use super::Styles;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The generated boxes an element had, keyed by the selector suffix naming each one.
pub type Pseudos = BTreeMap<String, Pseudo>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Pseudo {
    /// The `content` that generated this box, or empty when the engine generated it.
    ///
    /// `::before` and `::after` exist only because `content` produced something, so the
    /// value is the box's reason for existing and every rule must redeclare it. A
    /// `::backdrop` takes no `content` at all — the user agent generates it for a top-layer
    /// element — so it has none to carry, and emitting one would be a declaration the page
    /// never made.
    pub content: String,
    pub style: Styles,
}
