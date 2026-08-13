use super::{document_attribute, jsx_attribute};

/// React's complete namespaced family, transcribed from `possibleStandardNames.js` rather
/// than derived from the rule under test, so the test fails if the rule is wrong about any
/// member instead of agreeing with it by construction.
const NAMESPACED: &[(&str, &str)] = &[
    ("xlink:actuate", "xlinkActuate"),
    ("xlink:arcrole", "xlinkArcrole"),
    ("xlink:href", "xlinkHref"),
    ("xlink:role", "xlinkRole"),
    ("xlink:show", "xlinkShow"),
    ("xlink:title", "xlinkTitle"),
    ("xlink:type", "xlinkType"),
    ("xml:base", "xmlBase"),
    ("xml:lang", "xmlLang"),
    ("xml:space", "xmlSpace"),
    ("xmlns:xlink", "xmlnsXlink"),
];

/// A namespaced name is the one spelling JSX cannot express as a plain identifier.
/// `xlink:href` carries no hyphen to split on and is in no table, so it fell through every
/// gate to identity and was emitted verbatim. Babel rejects a `JSXNamespacedName` outright;
/// esbuild lowers it to a string-keyed prop, and React DOM reaches `setAttributeNS` only
/// from the camelCase spelling, so the attribute lands in no namespace and the `<use>` does
/// not resolve it.
///
/// Stated over React's whole family rather than the one name a page happened to use, and
/// pinned in the forward direction explicitly: the round trip cannot catch this, because
/// identity in both directions round-trips perfectly.
#[test]
fn namespaced_attributes_translate_to_their_react_props_in_both_directions() {
    for (document, jsx) in NAMESPACED {
        assert_eq!(
            jsx_attribute(document),
            *jsx,
            "{document} is not the React prop {jsx}"
        );
        assert_eq!(
            document_attribute(jsx),
            *document,
            "{jsx} is not restored to {document} for the XML output"
        );
    }
}

/// The invariant the fix has to establish, rather than the eleven names that motivated it:
/// nothing reaching a JSX attribute position may carry a colon, whatever the prefix. A page
/// built on Alpine or htmx carries colons React never named, and each one lands in no
/// namespace exactly as `xlink:href` does, so a table of the known family would leave them
/// broken.
///
/// The emptiness assertion refutes the cheap repair. Dropping every colon-bearing name also
/// clears the colon, while the attribute it addressed is silently gone — a worse outcome
/// than the wrong namespace, because the failure stops being visible.
#[test]
fn no_name_carries_a_colon_into_a_jsx_attribute_position() {
    for name in [
        "xlink:href",
        "xml:lang",
        "xmlns:xlink",
        "x-on:click",
        "hx-on::after-request",
    ] {
        let jsx = jsx_attribute(name);
        assert!(
            !jsx.contains(':'),
            "{name} kept a colon as {jsx}, which React's JSX transform rejects"
        );
        assert!(
            !jsx.is_empty(),
            "{name} was suppressed rather than translated"
        );
    }
}

/// `xmlns:xlink` is why the inverse matches a prefix followed by an uppercase letter rather
/// than the first prefix that matches. `xmlnsXlink` begins with `xml` too, so a bare prefix
/// search splits it into `xml:nsXlink` depending only on the order the list happens to be
/// written in.
#[test]
fn the_inverse_splits_a_namespace_on_the_word_boundary_not_the_first_prefix() {
    assert_eq!(document_attribute("xmlnsXlink"), "xmlns:xlink");
    // `xmlns` itself is not namespaced, and offers no word boundary to find.
    assert_eq!(jsx_attribute("xmlns"), "xmlns");
    assert_eq!(document_attribute("xmlns"), "xmlns");
    // A camelCase prop that merely begins with a listed prefix is not a namespace.
    assert_eq!(document_attribute("dateTime"), "datetime");
}
