//! The one reader of the shadow-root sentinel on the emitting side.
//!
//! A shadow root is a tree scope, not an element. The capture walk therefore records it under
//! a name in the *address* grammar — `::shadow-root(<mode>)`, with the tag `#shadow-root` —
//! which is not a tag name and cannot be written into JSX. Passing it through produces a file
//! that does not parse, so every node inside the tree is lost along with it.
//!
//! Translating it here, once, is what keeps the sentinel out of the artifact. Sanitising it
//! into a real element instead would be worse than the erasure: it asserts a parentage the
//! page never had, because the host's light children and the shadow tree would become
//! siblings in one scope and every shadow-scoped rule would apply document-wide.
//!
//! The emitted element opens a real shadow root at run time, so the browser computes the
//! flattened tree itself. That is the only form under which slots, `:host` and `::slotted()`
//! keep meaning something for a page nobody sampled.

use super::jsx_attrs::quoted;
use crate::model::Node;

/// The tag the capture walk gives a shadow root.
pub const TAG: &str = "#shadow-root";

/// The component the sentinel is translated into, and the module that supplies it.
pub const COMPONENT: &str = "ShadowRoot";
pub const MODULE: &str = "runtime/shadow.mjs";

/// Whether `node` is a shadow root rather than an element.
pub fn is_root(node: &Node) -> bool {
    node.tag == TAG
}

/// The mode of the tree an address opens, read off the sentinel segment that names it.
///
/// The mode is part of the address because a host may hold either kind and the two are
/// different tree scopes; a closed root is unreachable from `host.shadowRoot`, which is why
/// the runtime cannot recover the value by asking the host for it.
pub fn mode(path: &str) -> Option<&str> {
    path.rsplit('>')
        .next()?
        .strip_prefix("::shadow-root(")?
        .strip_suffix(')')
}

/// The opening and closing lines the sentinel becomes, wrapped around its rendered children.
pub fn element(node: &Node, children: &str, indent: &str) -> String {
    let mode = mode(&node.path).unwrap_or("open");
    format!(
        "{indent}<{COMPONENT} mode={}>\n{children}{indent}</{COMPONENT}>\n",
        quoted(mode)
    )
}

#[cfg(test)]
#[path = "shadow_root_tests.rs"]
mod tests;
