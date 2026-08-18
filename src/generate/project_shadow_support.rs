//! The shadow-root arm of the shared project fixture.

use super::project_test_support::node;
use crate::model::Node;

/// A host whose open shadow root holds one element and its text.
///
/// The shadow root is recorded under a name in the address grammar rather than a tag, so a
/// generator that writes it out emits a file that does not parse. That is invisible to every
/// spec-internal check, which is why it belongs in the fixture the whole-project tests read
/// rather than in a test of its own.
///
/// Two hosts, because one is not enough to reach the path that matters most: identical hosts
/// fingerprint alike, so a shadow root can be picked as a component root, and a component's
/// body is written from the node's tag with no translation anywhere near it.
pub(super) fn shadow_host(index: usize) -> Vec<Node> {
    let host = format!("html>body:nth-of-type(1)>x-shadow:nth-of-type({index})");
    let root = format!("{host}>::shadow-root(open)");
    let frame = format!("{root}>div:nth-of-type(1)");
    let mut nodes = vec![
        node(&host, Some("html>body:nth-of-type(1)"), "", None),
        node(&root, Some(&host), "", None),
        node(&frame, Some(&root), "", None),
        node(&format!("{frame}>#text(1)"), Some(&frame), "Shadowed", None),
    ];
    nodes[0].tag = "x-shadow".into();
    nodes[1].tag = super::shadow_root::TAG.into();
    nodes
}
