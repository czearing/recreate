//! The single owner of "what is this HTML attribute called in JSX?".
//!
//! Hyphenated attributes carry their own word boundaries, so `stroke-width` becomes
//! `strokeWidth` by a rule. A large family of HTML attributes does not: they are written as
//! one lowercase word while React's canonical prop is camelCase, and nothing in `datetime`
//! says the split falls after `date` rather than after `dat`. Those names are fixed points
//! of any hyphen-driven conversion, so the conversion cannot ever produce the right answer
//! for them and only a table can.
//!
//! Three of the family used to be hand-listed beside the conversion and the rest were not,
//! which made the omission invisible until a page happened to use one — a `<time>` element
//! emitted `datetime={...}`, a prop React does not recognise. The table below is therefore
//! the whole family rather than the cases seen so far.
//!
//! It is written as one list of canonical React names, not as pairs. The HTML spelling of
//! every entry is its own lowercase form, so the two halves cannot drift apart: adding a
//! name adds both directions at once.
//!
//! The same has to be true of the hyphenated family, and it was not. Converting to JSX
//! ended in a rule that answered for every name ever presented, while converting back was
//! a search bounded by `HYPHENATED`, so a name the list did not hold was camel-cased on the
//! way out and never restored on the way back — silently, since both halves succeeded. The
//! list cannot be completed: a presentation attribute is defined by reference to the CSS
//! property set, so the family grows with the styling module and any list transcribed today
//! is short tomorrow. Both directions therefore consult the list and an unknown name is
//! identity in both, which costs nothing where the two spellings already agree and makes an
//! omission harmless where they do not.
//!
//! A namespaced name is a third case. It is a fixed point of the hyphen rule for the same
//! reason `datetime` is — no hyphen to split on — but keeping it verbatim does not merely
//! emit a prop React ignores. React DOM reaches `setAttributeNS` only from the camelCase
//! spelling, so the colon-spelled name lands as a literal attribute in no namespace, and an
//! `xlink:href` outside the XLink namespace is one a `<use>` will not resolve. Here the
//! delimiter is present, so unlike the single-word family a rule can answer, and it answers
//! for every prefix rather than for the eleven React happens to name.

use super::jsx_attr_tables::{CAMEL_CASED, HYPHENATED, NAMESPACE_PREFIXES, RENAMED};

/// Translates a captured HTML attribute name into the prop name React recognises.
///
/// `aria-` and `data-` attributes are passed through verbatim: React accepts those exactly
/// as authored, and camel-casing them would emit a prop no renderer knows. Every other
/// unrecognised hyphenated name is passed through for the same reason — React writes a
/// hyphenated prop it does not know straight to the DOM, while an unrecognised camelCase
/// prop it drops, so the conversion is the lossy direction rather than the safe one.
///
/// That passthrough is sound for every name JSX can spell, and wrong for one it cannot. A
/// colon in an attribute position is a `JSXNamespacedName`. Babel rejects it outright; the
/// esbuild lowering this generator's own projects use accepts it and emits a string-keyed
/// prop instead. React DOM reaches `setAttributeNS` only from the camelCase spelling, so a
/// colon-spelled key misses every namespaced case and falls to the generic path, which
/// writes an attribute whose literal name contains a colon and which belongs to no
/// namespace at all — an `xlink:href` outside the XLink namespace, where a `<use>` will not
/// resolve it. Namespaces are therefore resolved first and unconditionally.
pub fn jsx_attribute(name: &str) -> String {
    if name.contains(':') {
        return delimited_to_camel(name, ':');
    }
    if name.starts_with("aria-") || name.starts_with("data-") {
        return name.into();
    }
    if let Some((_, renamed)) = RENAMED.iter().find(|(html, _)| *html == name) {
        return (*renamed).into();
    }
    if let Some(camel) = CAMEL_CASED
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(name))
    {
        return (*camel).into();
    }
    if HYPHENATED.contains(&name) {
        return hyphenated_to_camel(name);
    }
    name.into()
}

/// The inverse: the document spelling of a name React writes as a prop.
///
/// It reads the same lists as `jsx_attribute` rather than a table of its own, so the two
/// directions cannot drift apart — a private second copy is how `to_xml` came to rewrite
/// the `clipPath` element along with the `clipPath` attribute. Unknown names pass through
/// unchanged, because the far larger family is the one both spellings share.
pub fn document_attribute(name: &str) -> String {
    if let Some(namespaced) = namespaced_to_document(name) {
        return namespaced;
    }
    if let Some((document, _)) = RENAMED.iter().find(|(_, jsx)| *jsx == name) {
        return (*document).into();
    }
    if let Some(hyphenated) = HYPHENATED
        .iter()
        .find(|candidate| hyphenated_to_camel(candidate) == name)
    {
        return (*hyphenated).into();
    }
    if CAMEL_CASED.contains(&name) {
        return name.to_ascii_lowercase();
    }
    name.into()
}

/// Restores the colon a namespaced prop was camel-joined across, for the XML the relocated
/// SVG assets are written as. `None` for every name that is not namespaced, which is the
/// overwhelming majority and must be left alone.
fn namespaced_to_document(name: &str) -> Option<String> {
    let prefix = NAMESPACE_PREFIXES.iter().find(|prefix| {
        name.strip_prefix(**prefix)
            .and_then(|local| local.chars().next())
            .is_some_and(|first| first.is_ascii_uppercase())
    })?;
    let local = &name[prefix.len()..];
    Some(format!("{prefix}:{}", local.to_ascii_lowercase()))
}

fn hyphenated_to_camel(value: &str) -> String {
    delimited_to_camel(value, '-')
}

/// Joins the words of a delimited name into camelCase. The two delimiters that reach an
/// attribute name mean different things — a hyphen separates words, a colon separates a
/// namespace from a local name — but the spelling React wants is the same operation, so it
/// is written once rather than once per delimiter.
fn delimited_to_camel(value: &str, delimiter: char) -> String {
    let mut result = String::new();
    let mut uppercase = false;
    for character in value.chars() {
        if character == delimiter {
            uppercase = true;
        } else if uppercase {
            result.extend(character.to_uppercase());
            uppercase = false;
        } else {
            result.push(character);
        }
    }
    result
}

#[cfg(test)]
#[path = "jsx_attr_names_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "jsx_attr_namespace_tests.rs"]
mod namespace_tests;
