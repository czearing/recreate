//! The single owner of converting generated JSX markup back into a document.
//!
//! An extracted SVG is written twice: once as JSX inside a component, and once as a
//! standalone `.svg` asset. The second spelling used to be produced by folding a rename
//! table over the flat string with `str::replace`, which is position-blind. `clipPath` is
//! both an SVG element and a React attribute, so the rename that correctly produced
//! `clip-path="url(#c)"` also rewrote `<clipPath>` and `</clipPath>` into an element that
//! does not exist, and every shape referencing the definition lost its clip silently.
//!
//! The rule that closes it is not an anchor around that one name. A rewrite belongs to a
//! syntactic category, so the converter must know which category it is in. Reading by
//! position makes closing-tag balance, values such as `id="clipPath1"`, and every future
//! name that collides with an element correct by construction rather than by a conditional
//! each. `jsx_markup_scan` supplies the position; this module supplies the meaning.

use super::{
    jsx_attr_names::document_attribute,
    jsx_markup_scan::{Token, Value, scan},
};
use std::collections::BTreeSet;

/// The qualified names `to_xml` writes, split by the two scopes a reader can need.
///
/// Whether a name is *used* is a question about the whole graphic, because a declaration on
/// the root has to cover every descendant. Whether a name is already *declared* is a
/// question about one element, because a declaration belongs to the tag that writes it.
/// Collapsing the two is what lets a descendant answer for its ancestor.
#[derive(Default)]
pub(super) struct Names {
    /// Every element and attribute name in the subtree, in the document's spelling.
    pub used: BTreeSet<String>,
    /// The attribute names on the root element's own start tag.
    pub root: BTreeSet<String>,
    /// The root element's own name, which is what a caller needs to close it.
    pub root_element: String,
}

/// The XML spelling of generated JSX markup: element names verbatim, attribute names
/// translated back to the document spelling, and serialized values unwrapped.
pub(super) fn to_xml(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    scan(source, |token| match token {
        Token::Text(text) => output.push_str(text),
        Token::Literal(body) => output.push_str(&escape(&unescape(body))),
        Token::Open { closing, name } => {
            output.push('<');
            if closing {
                output.push('/');
            }
            output.push_str(name);
        }
        Token::Attribute { name, value } => {
            output.push(' ');
            output.push_str(&document_attribute(name));
            match value {
                Some(Value::Literal(body)) => {
                    output.push_str(&format!("=\"{}\"", escape(&unescape(body))));
                }
                Some(Value::Expression(body)) => output.push_str(&format!("=\"{body}\"")),
                None => {}
            }
        }
        Token::Close { self_closing } => {
            output.push_str(if self_closing { " />" } else { ">" });
        }
    });
    output
}

/// The names `to_xml` will write, by scope. Reading them from the emitted XML instead would
/// mean a second parser for a second grammar, and the two would drift; reading them from the
/// same source through the same scanner and the same `document_attribute` makes them the
/// names that are actually emitted, by construction.
pub(super) fn qualified_names(source: &str) -> Names {
    let mut names = Names::default();
    scan_scoped(source, |token, root| match token {
        Token::Open { name, .. } => {
            if root && names.root_element.is_empty() {
                names.root_element = name.into();
            }
            names.used.insert(name.into());
        }
        Token::Attribute { name, .. } => {
            let name = document_attribute(name);
            if root {
                names.root.insert(name.clone());
            }
            names.used.insert(name);
        }
        _ => {}
    });
    names
}

/// Every value bound to `name` as an attribute, in document order, unescaped. Reading
/// these by searching for `name={"` would also match the letters wherever they appear in
/// page content, inventing bindings the document never carried.
pub(super) fn attribute_values(source: &str, name: &str) -> Vec<String> {
    let mut values = Vec::new();
    scan(source, |token| {
        if let Token::Attribute {
            name: found,
            value: Some(Value::Literal(body)),
        } = token
            && found == name
        {
            values.push(unescape(body));
        }
    });
    values
}

/// The value `name` is bound to on the **root element's own start tag**, unescaped.
///
/// A serialized subtree is not an element. `attribute_values` answers "anywhere in this
/// markup", so taking its first match reads a descendant's value whenever the root omits
/// the name — and first-in-document-order coincides with belongs-to-the-root exactly
/// while the root declares it, which hides the divergence behind every case where the
/// answer was never in doubt. What is wrong with a harvested value is its provenance, not
/// its shape, so no downstream guard on the value itself can recover the distinction; the
/// scope has to come from the tag boundary. That boundary is `Open`..`Close`, which the
/// scanner already tracks through quoting, so a value containing `>` cannot cut the root
/// tag short and start silently dropping the attributes the root really declared.
pub(super) fn root_attribute(source: &str, name: &str) -> Option<String> {
    let mut value = None;
    scan_scoped(source, |token, root| {
        if let Token::Attribute {
            name: found,
            value: Some(Value::Literal(body)),
        } = token
            && root
            && found == name
            && value.is_none()
        {
            value = Some(unescape(body));
        }
    });
    value
}

/// The single owner of that boundary: every token, paired with whether it belongs to the
/// root element's own start tag. Two readers now ask the question, and a scope rule written
/// twice is a scope rule that will be corrected once.
fn scan_scoped<'a>(source: &'a str, mut emit: impl FnMut(Token<'a>, bool)) {
    let mut inside_root = false;
    let mut root_seen = false;
    scan(source, |token| {
        if !root_seen && matches!(token, Token::Open { closing: false, .. }) {
            inside_root = true;
            root_seen = true;
        }
        let root = inside_root;
        if matches!(token, Token::Close { .. }) {
            inside_root = false;
        }
        emit(token, root);
    });
}

fn unescape(body: &str) -> String {
    serde_json::from_str(&format!("\"{body}\"")).unwrap_or_else(|_| body.into())
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
#[path = "jsx_markup_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "jsx_markup_scope_tests.rs"]
mod scope_tests;
