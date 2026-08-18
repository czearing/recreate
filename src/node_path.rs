//! The one definition of an element's path, and the only way a script receives it.
//!
//! Every in-page pass that records or reads a node key resolves it through this fragment, so
//! a page's element has one address whichever pass met it first. The fragment is statements
//! only and closes over nothing but platform globals, which is what lets the same text be
//! injected into an async IIFE, an arrow-function body and a bare expression alike.

const SOURCE: &str = include_str!("node_path.js");

/// Substitutes the shared definition into `script`.
///
/// Every caller goes through this rather than pasting the text, so the fragment cannot be
/// half-updated: there is one definition and one substitution.
pub fn embed(script: &str) -> String {
    script.replace(PLACEHOLDER, SOURCE)
}

/// Reports whether a script already carries the shared definition, which is what lets a test
/// pin the seam rather than the behaviour a second copy would also satisfy.
#[cfg(test)]
pub fn embedded(script: &str) -> bool {
    script.contains(SOURCE.trim_end())
}

pub const PLACEHOLDER: &str = "__NODE_PATH__";

#[cfg(test)]
#[path = "node_path_tests.rs"]
mod tests;
