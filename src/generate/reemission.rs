//! Which captured elements may be written back out, and which must not be.
//!
//! Both emitters ask this module before re-serialising a captured element, because either
//! can reach any element: `document` writes the head into the shell, and `structural_tree`
//! registers everything else for the render walk. A rule enforced on one of them restores
//! the head case and leaves the body case broken, which is how one defect arrives twice.
//! Keeping the judgement here, rather than beside whichever emitter first needed it, is
//! what makes the single owner structural instead of conventional.

use crate::model::Node;

/// Whether re-serialising an element would act on the recreation rather than describe the
/// page it reproduces.
///
/// The members share a subject, not a symptom. An element in this set says something whose
/// meaning is fixed by the document containing it, so copying the element into a different
/// document does not reproduce a statement — it re-runs an effect somewhere it was never
/// aimed. The recreation is always the wrong target: it ships a bundler entry, injects its
/// own styles and serves from another origin, so it needs strictly more latitude than the
/// page it reproduces, and the error only ever runs one way. Nothing is lost by refusing;
/// the captured state keeps the element verbatim, which is where the record belongs.
pub(super) fn acts_on_its_document(node: &Node) -> bool {
    supplies_css(node) || instructs_user_agent(node)
}

/// Whether an element instructs the user agent loading the document.
///
/// HTML partitions `<meta>` by attribute, and the forms are mutually exclusive: `charset`
/// declares an encoding, `name` and `itemprop` carry document metadata, and `http-equiv` is
/// a pragma directive — an HTTP response header written in markup. Only the last has the
/// loading document as its subject, so only the last changes meaning when it is copied onto
/// a different one. `document::render` writes the head ahead of the entry
/// `<script type="module">`, and a meta-delivered policy governs everything that follows it,
/// so a captured `script-src 'none'` forbids the artifact's own entry point and nothing
/// renders — with no error event, because a blocked script does not fire one.
///
/// Keyed on the attribute rather than on its value because the pragma set is open and
/// versioned: a list of hazardous directives is short the day it is written and fails open
/// on the next one, while refusing the category fails closed. No description is lost —
/// `charset` and `name` are different attributes in that same partition — and none is
/// invented to compensate, because the encoding a document obeys comes from the HTTP
/// `Content-Type`, which outranks any declaration inside it.
fn instructs_user_agent(node: &Node) -> bool {
    node.tag == "meta" && node.attributes.contains_key("http-equiv")
}

/// Whether an element delivers authored CSS, by any route.
///
/// Every authored rule already reaches the output through `css_base`, which re-emits from
/// the captured `css_rules` under `css::global_rule` — the gate that exists because the
/// bake already represents any rule that reached an element through a selector. Emitting
/// a `<style>` verbatim, or leaving a stylesheet `<link>` live, routes around that gate
/// and applies those rules a second time, at whatever specificity they were authored
/// with. So which declaration wins would depend on how the page happened to ship it, and
/// an authored rule the original cascade rejected can outrank the value that beat it.
/// Rejecting the delivery instead of filtering its text keeps one owner for the decision.
fn supplies_css(node: &Node) -> bool {
    node.tag == "style"
        || (node.tag == "link"
            && node.attributes.get("rel").is_some_and(|relation| {
                relation
                    .split_ascii_whitespace()
                    .any(|token| token.eq_ignore_ascii_case("stylesheet"))
            }))
}

/// Whether a head child may be re-emitted into the shell.
pub(super) fn safe_head_node(node: &Node) -> bool {
    if acts_on_its_document(node) {
        return false;
    }
    if node.tag != "link" {
        return matches!(node.tag.as_str(), "base" | "meta" | "title");
    }
    let relation = node.attributes.get("rel").map(String::as_str);
    let kind = node.attributes.get("as").map(String::as_str);
    let href = node.attributes.get("href").map(String::as_str);
    safe_link(relation, kind) && resolvable_link(href)
}

/// A relative href names a file in the source site's own build output, which the
/// recreation never produces, so keeping it only yields a guaranteed 404. Absolute
/// and data hrefs still resolve, so they are kept.
fn resolvable_link(href: Option<&str>) -> bool {
    let Some(href) = href.map(str::trim) else {
        return false;
    };
    href.starts_with("data:") || href.starts_with("//") || {
        let scheme = href.split_once("://").map(|(scheme, _)| scheme);
        matches!(scheme, Some("http") | Some("https"))
    }
}

fn safe_link(relation: Option<&str>, kind: Option<&str>) -> bool {
    relation != Some("modulepreload") && !(relation == Some("preload") && kind == Some("script"))
}

#[cfg(test)]
#[path = "document_link_tests.rs"]
mod link_tests;

#[cfg(test)]
#[path = "head_directive_tests.rs"]
mod directive_tests;
