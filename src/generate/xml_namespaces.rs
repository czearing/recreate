//! The single owner of "which namespace prefixes must this document declare?".
//!
//! Inline SVG inside HTML declares nothing. The HTML parser assigns the SVG namespace by
//! element name and maps a fixed table of foreign attributes — `xlink:*`, `xml:*` — onto
//! their real namespaces while it builds the tree, so a prefix there is *recognised*, never
//! resolved, and no declaration is ever consulted. Relocated into a standalone `.svg`, the
//! identical bytes meet the XML parser instead, which resolves a prefix only through an
//! in-scope `xmlns:` declaration and must halt on one it cannot bind. Nothing was edited;
//! the parser changed. That is why the loss is invisible to any comparison of the markup
//! against its source, and why it is fatal to the whole document rather than to one link.
//!
//! The rule is therefore not "supply the SVG namespace". It is: **every prefix appearing on
//! a name in this document is bound on its root**. The default namespace is the first case
//! of that rule, not a sibling of it — a second literal beside it would leave the next
//! prefix to be discovered by another blank image.
//!
//! Root-hoisting is safe here precisely because the subtree arrives as a finished string,
//! so every name is knowable before a byte is written. A descendant that redeclares a
//! prefix still wins inside itself, so a root declaration can only ever bind a name that
//! was otherwise unbound.

use super::jsx_markup::Names;
use std::collections::{BTreeMap, BTreeSet};

const DEFAULT: &str = "http://www.w3.org/2000/svg";

/// The prefixes HTML's *adjust foreign attributes* step genuinely namespaces, and the URIs
/// it binds them to. Reproducing that table is what keeps the relocated copy meaning what
/// the original meant; inventing a URI for `xlink` would produce a document that parses and
/// links to nothing.
const BOUND: &[(&str, &str)] = &[("xlink", "http://www.w3.org/1999/xlink")];

/// Prefixes the XML Namespaces specification binds itself. `xml` is bound implicitly and
/// declaring it is only legal against its one true URI; `xmlns` must never be declared at
/// all. They are listed together because the trap is the same: a sweep for colon-bearing
/// names collects `xmlns:xlink` — a *declaration* — and reports it as a prefix in use,
/// which would emit `xmlns:xmlns` and break exactly the documents that were already right.
const RESERVED: &[&str] = &["xml", "xmlns"];

/// Binds, on the root start tag, every prefix this document's own names use and its root
/// does not already declare.
pub(super) fn declare(mut xml: String, names: &Names) -> String {
    let declared = declared(&names.root);
    let mut missing = BTreeMap::new();
    if !declared.contains("") {
        missing.insert(String::new(), DEFAULT.to_string());
    }
    for prefix in used(&names.used) {
        if !declared.contains(prefix.as_str()) {
            let uri = uri(&prefix);
            missing.insert(prefix, uri);
        }
    }
    let declarations = missing
        .iter()
        .map(|(prefix, uri)| match prefix.is_empty() {
            true => format!(" xmlns=\"{uri}\""),
            false => format!(" xmlns:{prefix}=\"{uri}\""),
        })
        .collect::<String>();
    if let Some(index) = root_name_end(&xml) {
        xml.insert_str(index, &declarations);
    }
    xml
}

/// The prefixes the root start tag already binds, the default namespace reading as the
/// empty prefix. This is deliberately the *root's own* attributes and not the serialized
/// subtree: a declaration belongs to the element that writes it, so a `<foreignObject>`
/// child declaring XHTML says nothing about what its ancestor needs. Asking the flat string
/// instead lets any descendant starve the root of the declaration it requires.
fn declared(root: &BTreeSet<String>) -> BTreeSet<&str> {
    root.iter()
        .filter_map(|name| match name.as_str() {
            "xmlns" => Some(""),
            other => other.strip_prefix("xmlns:"),
        })
        .collect()
}

/// The prefixes carried by names anywhere in the subtree, which is the scope a root
/// declaration has to cover. A name with more than one colon is not a qualified name at all
/// and no declaration can repair it, so it is left as it stands rather than mined for a
/// prefix that would bind nothing.
fn used(names: &BTreeSet<String>) -> BTreeSet<String> {
    names
        .iter()
        .filter(|name| name.matches(':').count() == 1)
        .filter_map(|name| name.split(':').next())
        .filter(|prefix| !prefix.is_empty() && !RESERVED.contains(prefix))
        .map(str::to_string)
        .collect()
}

/// A namespace name is opaque and never dereferenced, so an unrecognised prefix does not
/// need a URI anyone has heard of — it needs *a* URI. Binding it keeps the qualified name
/// and its value exactly as authored while leaving it inert for rendering; dropping the
/// name to satisfy the parser would be an edit to the document, which relocation may not
/// make.
fn uri(prefix: &str) -> String {
    BOUND
        .iter()
        .find(|(known, _)| *known == prefix)
        .map(|(_, uri)| (*uri).to_string())
        .unwrap_or_else(|| format!("urn:recreate:ns:{prefix}"))
}

/// The offset just past the root element's name, where a declaration is legal and where no
/// attribute already written can be split in half.
fn root_name_end(xml: &str) -> Option<usize> {
    let start = xml.find('<')?;
    let name = &xml[start + 1..];
    let length = name.find(['/', '>', ' ', '\t', '\n', '\r'])?;
    Some(start + 1 + length)
}
